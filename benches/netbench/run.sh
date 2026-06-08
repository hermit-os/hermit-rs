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

case "$mode" in
    tcp-server-bw) bin="tcp-bw"; subcmd="server"; args="--bytes 1048576 --rounds 1000" ;;
    tcp-client-bw) bin="tcp-bw"; subcmd="client"; args="--bytes 1048576 --rounds 1000" ;;
    udp-server-bw) bin="udp-bw"; subcmd="server"; args="--bytes 1472 --rounds 1000" ;;
    udp-client-bw) bin="udp-bw"; subcmd="client"; args="--bytes 1472 --rounds 1000" ;;
    tcp-server-latency) bin="tcp-latency"; subcmd="server"; args="--bytes 64 --rounds 100000" ;;
    tcp-client-latency) bin="tcp-latency"; subcmd="client"; args="--bytes 64 --rounds 100000" ;;
    udp-server-latency) bin="udp-latency"; subcmd="server"; args="--bytes 64 --rounds 100000" ;;
    udp-client-latency) bin="udp-latency"; subcmd="client"; args="--bytes 64 --rounds 100000" ;;
    *)
        echo "Unknown benchmark: $mode" >&2
        exit 1
        ;;
esac

hermit() {
    echo "Building $bin image"

    cargo build --manifest-path "$netbench_dir"/Cargo.toml --bin $bin \
        -Zbuild-std=core,alloc,std,panic_abort -Zbuild-std-features=compiler-builtins-mem \
        --target x86_64-unknown-hermit \
        --features hermit/virtio-net \
        --release

    echo "Launching $bin image on QEMU"

    sudo qemu-system-x86_64 -cpu host \
            -enable-kvm -display none -smp 1 -m 1G -serial stdio \
            -kernel "$root_dir"/kernel/hermit-loader-x86_64 \
            -initrd "$root_dir"/target/x86_64-unknown-hermit/release/$bin \
            -netdev tap,id=net0,script="$root_dir"/kernel/xtask/hermit-ifup,vhost=on \
            -device virtio-net-pci,netdev=net0,disable-legacy=on \
            -append "-- $subcmd --address 10.0.5.1 $args"
}

linux() {
    echo "Launching $bin on linux"

    cargo run --manifest-path "$netbench_dir"/Cargo.toml --bin $bin \
        --release \
        --target x86_64-unknown-linux-gnu \
        -- \
        $subcmd --address 10.0.5.3 $args
}

$1
