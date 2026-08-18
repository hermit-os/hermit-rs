#!/usr/bin/env bash

# Usage:   run.sh TARGET MODE
# Example: run.sh linux tcp-server-bw
#          run.sh hermit tcp-client-bw
#          run.sh linux udp-server-bw
#          run.sh hermit udp-client-bw
#          run.sh linux tcp-server-latency
#          run.sh hermit tcp-client-latency
#          run.sh linux udp-server-latency
#          run.sh hermit udp-client-latency

set -o errexit

netbench_dir="${0%/*}"
root_dir="$netbench_dir"/../..

mode=$2
bin="rust-tcp-io-perf"
arch="$(uname -m)"

# Fortunately the loader uses the same naming as uname -m does
loader_arch="$arch"

case "$arch" in
    aarch64) rust_arch="aarch64"; qemu_arch="aarch64"; qemu_machine="virt" ;;
    aarch64_be) rust_arch="aarch64_be"; qemu_arch="aarch64"; qemu_machine="virt" ;;
    riscv64) rust_arch="riscv64gc"; qemu_arch="riscv64"; qemu_machine="virt" ;;
    x86_64) rust_arch="x86_64"; qemu_arch="x86_64"; qemu_machine="pc" ;;
    *)
        echo "Unsupported architecture: $arch" >&2
        exit 1
        ;;
esac

case "$mode" in
    tcp-server-bw) benchmark="bw"; protocol="tcp"; subcmd="server"; args="--bytes 1048576 --rounds 1000" ;;
    tcp-client-bw) benchmark="bw"; protocol="tcp"; subcmd="client"; args="--bytes 1048576 --rounds 1000" ;;
    udp-server-bw) benchmark="bw"; protocol="udp"; subcmd="server"; args="--bytes 1472 --rounds 1000" ;;
    udp-client-bw) benchmark="bw"; protocol="udp"; subcmd="client"; args="--bytes 1472 --rounds 1000" ;;
    tcp-server-latency) benchmark="latency"; protocol="tcp"; subcmd="server"; args="--bytes 64 --rounds 100000" ;;
    tcp-client-latency) benchmark="latency"; protocol="tcp"; subcmd="client"; args="--bytes 64 --rounds 100000" ;;
    udp-server-latency) benchmark="latency"; protocol="udp"; subcmd="server"; args="--bytes 64 --rounds 100000" ;;
    udp-client-latency) benchmark="latency"; protocol="udp"; subcmd="client"; args="--bytes 64 --rounds 100000" ;;
    *)
        echo "Unknown benchmark: $mode" >&2
        exit 1
        ;;
esac

hermit() {
    echo "Building $bin image"

    cargo build --manifest-path "$netbench_dir"/Cargo.toml \
        -Zbuild-std=core,alloc,std,panic_abort -Zbuild-std-features=compiler-builtins-mem \
        --target "$rust_arch-unknown-hermit" \
        --features hermit/loader,hermit/virtio-net \
        --release

    echo "Launching $bin image on QEMU"

    # QEMU on aarch64 and riscv64 parses the file passed via -kernel and assumes
    # that anything that is ELF is not a Linux kernel, which makes it not process
    # the -initrd argument

    kernel="$root_dir"/target/"$rust_arch-unknown-hermit"/release/"$bin"
    case "$arch" in
        aarch64*|riscv64) initrd_arg="-device guest-loader,addr=0x48000000,initrd=$kernel" ;;
        x86_64) initrd_arg="-initrd $kernel" ;;
    esac

    case "$arch" in
        aarch64*) loader_suffix=elf ;;
        riscv64) loader_suffix=sbi ;;
        x86_64) loader_suffix=multiboot ;;
    esac

    sudo "qemu-system-$qemu_arch" -M "$qemu_machine" -cpu host \
            -enable-kvm -display none -smp 1 -m 1G -serial stdio \
            -kernel "$root_dir"/kernel/"hermit-loader-$loader_arch-$loader_suffix" \
            $initrd_arg \
            -netdev tap,id=net0,script="$root_dir"/kernel/xtask/hermit-ifup,vhost=on \
            -device virtio-net-pci,netdev=net0,disable-legacy=on \
            -append "-- $benchmark $protocol $subcmd --address 10.0.5.1 $args"
}

linux() {
    echo "Launching $bin on linux"

    cargo run --manifest-path "$netbench_dir"/Cargo.toml \
        --release \
        -- \
        $benchmark $protocol $subcmd --address 10.0.5.3 $args
}

$1
