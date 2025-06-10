# Hermit-WASM - Running WASM modules inside a lightweight VM

[![Crates.io](https://img.shields.io/crates/v/hermit-wasm.svg)](https://crates.io/crates/hermit-wasm)
[![License](https://img.shields.io/crates/l/hermit-wasm.svg)](https://img.shields.io/crates/l/hermit-wasm.svg)

_Hermit-WASM_ is able to run WASM Modules on top of the Unikernel [Hermit](https://hermit-os.org/) inside a lightweight virtual machine. Its purpose is to enable applications to safely run untrusted or third party WASM code within a VM with very low latency/overhead.

The current version of _Hermit-WASM_ requires the Rust's nightly compiler and is a prototype, which just supports the target [wasm32-wasip1](https://doc.rust-lang.org/rustc/platform-support/wasm32-wasip1.html). In addition, _Hermit-WASM_ realizes only a subset of the required [bindings](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md).

## Requirements

* [`rustup`](https://www.rust-lang.org/tools/install)
* Install required toolchain: `rustup target add wasm32-wasip1`

## Building from source

To build from source, simply checkout the code and use `cargo build` with a hermit target. The following commands build _Hermit-WASM_ for _aarch64_ processors:

```sh
# clone Hermit repository
git clone --recurse-submodules https://github.com/hermit-os/hermit-rs.git
# switch the directory of the Hermit repository
cd hermit-rs
# build Hermit-WASM
cargo build -Zbuild-std=std,panic_abort -Zbuild-std-features=compiler-builtins-mem --target aarch64-unknown-hermit -p hermit-wasm --release
```

To build _Hermit-WASM_ for other architecture, replace _aarch64-unknown-hermit_ by _x86_64-unknown-hermit_ for the x86 architecture or _riscv64gc-unknown-hermit_ for RISC-V architecture.

## Usage

This guideline assumes that Linux is used as host operating system on top of aarch64 processor and [virtiofsd](https://virtio-fs.gitlab) is installed. In addition, the host offers KVM to accelerate the virtual machine.

Build demo application _wasm-test_ for the target _wasm32-wasip1_.
```sh
cargo build --target wasm32-wasip1  --release -p wasm-test
```

If Qemu is used as hypervisor, download the loader binary from its [releases page](https://github.com/hermit-os/loader/releases).
Use _virtiofsd_ to provide the target directory for _Hermit-WASM_.
```sh
virtiofsd --socket-path=./vhostqemu --shared-dir ./target/wasm32-wasip1/release --announce-submounts --sandbox none --seccomp none --inode-file-handles=never
```

Start _Hermit-WASM_ within the hypervisor Qemu as followed:
```sh
qemu-system-aarch64 --enable-kvm -display none -serial stdio -kernel hermit-loader-x86_64 -initrd target/aarch64-unknown-hermit/release/hermit-wasm -append "-- /root/wasm-test.wasm" -cpu host -device isa-debug-exit,iobase=0xf4,iosize=0x04 -smp 1 -m 2G -global virtio-mmio.force-legacy=off -chardev socket,id=char0,path=./vhostqemu -device vhost-user-fs-pci,queue-size=1024,packed=on,chardev=char0,tag=root -object memory-backend-file,id=mem,size=1024M,mem-path=/dev/shm,share=on -numa node,memdev=mem
```

As alternative, [uhyve](https://github.com/hermit-os/uhyve) can be used, which is a minimal hypervisor for Hermit and offers direct access to a local directory. Consequently, uhyve doesn't depend on _virtiofsd_. In the following example, a local file is mounted to _/root/wasm-test.wasm_.
```sh
uhyve -c 1 -m 1GiB --file-isolation none --file-mapping target/wasm32-wasip1/release/wasm-test.wasm:/root/wasm-test.wasm target/aarch64-unknown-hermit/release/hermit-wasm -- -- /root/wasm-test.wasm
```

## Credits

A similar project is this area is [Hyperlight-Wasm](https://github.com/hyperlight-dev/hyperlight-wasm). As far as known, _Hyperlight-Wasm_ supports only _x86_ systems, while _Hermit-WASM_ is also running on _aarch64_ and _RISC-V_ processors.

## Licensing

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

Hermit-WASM is being developed on [GitHub](https://github.com/hermit-os/hermit-rs/examples/hermit-wasm).
Create your own fork, send us a pull request, and chat with us on [Zulip](https://hermit.zulipchat.com/).
