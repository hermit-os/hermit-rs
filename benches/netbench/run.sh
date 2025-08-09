#!/usr/bin/env bash

# Usage:   run.sh TARGET BIN [--rftrace] [--user-networking]
# Example: run.sh linux tcp-server-bw
#          run.sh hermit tcp-client-bw

set -o errexit

rftrace=false
user_networking=false
target=
bin=

while [[ $# -gt 0 ]]; do
	case "$1" in
	--rftrace)
		rftrace=true
		shift
		;;
	--user-networking)
		user_networking=true
		shift
		;;
	-*)
		echo "Unknown option: $1" >&2
		exit 1
		;;
	*)
		if [[ -z "$target" ]]; then
			target="$1"
		elif [[ -z "$bin" ]]; then
			bin="$1"
		else
			echo "Unexpected argument: $1" >&2
			exit 1
		fi
		shift
		;;
	esac
done

netbench_dir="${0%/*}"
root_dir="$netbench_dir"/../..

args="--bytes 1048576 --rounds 1000"

hermit() {
	echo "Building $bin image"

	rustflags=

	cargo_cmd=(
		cargo build
		--manifest-path "$netbench_dir"/Cargo.toml
		--bin "$bin"
		-Zbuild-std=core,alloc,std,panic_abort
		-Zbuild-std-features=compiler-builtins-mem
		--target "$(uname -m)-unknown-hermit"
		--profile release-debug
	)

	features=()

	if $rftrace; then
		features+=(rftrace)
		rustflags="-Zinstrument-mcount"

		mkdir -p tracedir

		sudo /usr/libexec/virtiofsd \
			--socket-path=/tmp/vhostqemu \
			--shared-dir=$(pwd)/tracedir \
			--announce-submounts \
			--sandbox none \
			--seccomp none \
			--inode-file-handles=never &
		sleep 1

		sudo chmod 777 /tmp/vhostqemu
	fi

	if $user_networking; then
		features+=(dhcp)
	fi

	features_str="${features[*]}"
	cargo_cmd+=(--features "$features_str")

	RUSTFLAGS="$rustflags" "${cargo_cmd[@]}"

	echo "Launching $bin image on QEMU"

	qemu_cmd=(
		qemu-system-$(uname -m)
		-cpu host
		-enable-kvm
		-display none
		-smp 1
		-m 1G
		-serial stdio
		-kernel "$root_dir/kernel/hermit-loader-$(uname -m)"
		-initrd "$root_dir/target/$(uname -m)-unknown-hermit/release-debug/$bin"
	)

	if $rftrace; then
		qemu_cmd+=(
			-chardev socket,id=char0,path=/tmp/vhostqemu
			-device vhost-user-fs-pci,queue-size=1024,packed=on,chardev=char0,tag=tracedir
			-object memory-backend-file,id=mem,size=1G,mem-path=/dev/shm,share=on
			-numa node,memdev=mem
		)
	fi

	if $user_networking; then
		qemu_cmd+=(
			-netdev user,id=net0,hostfwd=tcp::9975-:9975,hostfwd=udp::9975-:9975,net=192.168.76.0/24,dhcpstart=192.168.76.9
			-device virtio-net-pci,netdev=net0,disable-legacy=on
			-append "-- --address 127.0.0.1 $args"
		)
	else
		qemu_cmd+=(
			-netdev tap,id=net0,ifname=tap10,script=no,downscript=no,vhost=on
			-device virtio-net-pci,netdev=net0,disable-legacy=on
			-append "-- --address 10.0.5.1 $args"
		)
	fi

	sudo "${qemu_cmd[@]}"

	if $rftrace; then
		sleep 1
		nm -n "$root_dir"/target/$(uname -m)-unknown-hermit/release-debug/$bin >tracedir/$bin.sym
		uftrace dump -d tracedir --flame-graph >tracedir/flamegraph.txt
		flamegraph.pl tracedir/flamegraph.txt >tracedir/flamegraph.svg
		firefox tracedir/flamegraph.svg
	fi
}

linux() {
	echo "Launching $bin on linux"

	if $user_networking; then
		address=127.0.0.1
	else
		address=10.0.5.3
	fi

	cargo run --manifest-path "$netbench_dir"/Cargo.toml --bin $bin \
		--release \
		--target $(uname -m)-unknown-linux-gnu \
		-- \
		--address $address $args
}

$target
