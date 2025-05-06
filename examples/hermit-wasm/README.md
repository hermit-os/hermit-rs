# Hermit-WASM - Running WASM modules inside a lightweight VM

Hermit-Wasm is able to run WASM Modules on top of the Unikernel [Hermit](https://hermit-os.org/) inside a lightweight virtual machine. Its purpose is to enable applications to safely run untrusted or third party Wasm code within a VM with very low latency/overhead.

The current version of _Hermit-WASM_ requires the Rust's nightly compiler and is a prototype, which just supports the target [wasm32-wasip1](https://doc.rust-lang.org/rustc/platform-support/wasm32-wasip1.html) and [wasm32-wasip1-threads](https://doc.rust-lang.org/rustc/platform-support/wasm32-wasip1-threads.html). In addition, _Hermit-WASM_ realizes only a subset of the required [bindings](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md).

## Requirements

* [`rustup`](https://www.rust-lang.org/tools/install)
* Install required toolchains: `rustup target add wasm32-wasip1`

## Building from source

To build from source, simply checkout the code and use `cargo build` with a hermit target. The following commands build _Hermit-WASM_ for a _x86_64_ processor:

```sh
# clone Hermit repository
git clone --recurse-submodules https://github.com/hermit-os/hermit-rs.git
# build Hermit-WASM
cargo build -Zbuild-std=std,panic_abort -Zbuild-std-features=compiler-builtins-mem --target aarch64-unknown-hermit -p hermit-wasm --release
```

To build _Hermit-WASM_ for other architecture, replace _x86_64-unknown-hermit_ by _aarch64-unknown-hermit_ for the x86 architecture or _riscv64gc-unknown-hermit_ for RISC-V architecture.

## Usage

This guideline assumes that Linux is used as host operating system on top of x86_64 processor. In addition, the host offers KVM to accelerate the virtual machine.

If Qemu is used as hypervisor, download the loader binary from its [releases page](https://github.com/hermit-os/loader/releases) and start the hypervisor as followed:

```sh
qemu-system-x86_64 --enable-kvm -display none -serial stdio -kernel hermit-loader-x86_64 -cpu host -device isa-debug-exit,iobase=0xf4,iosize=0x04 -smp 1 -m 2G  -initrd path_to_hermit-wasm
```

_path_to_hermit_wasm_ should be replaced by your local path to the binary. Without any arguments, _Hermit-WASM_ starts a [WASM module](https://github.com/hermit-os/hermit-rs/tree/main/examples/wasm-test), which is included in the binary.

To load a WASM module from the file system, a local directory has to be mounted within the virtual machine.
_Hermit_ supports the usage of [virtiofsd](https://github.com/hermit-os/hermit-rs/wiki/Advanced-Configuration-Features#using-virtiofs-to-share-a-file-system-only-required-when-using-qemu) to mount a local directory.

As alternative, [uhyve](https://github.com/hermit-os/uhyve) can be used, which offers direct access to a local directory. In the following example, a local file is mounted to _/root/module.wasm_.

```sh
uhyve -c 1 -m 1GiB --file-isolation none --file-mapping path_to_module.wasm:/root/module.wasm target/x86_64-unknown-hermit/release/hermit-wasm -- -- -f /root/module.wasm
```

In this example, _path_to_module.wasm_ should be also replace by a local path to a WASM module.

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
