<img width="100" align="right" src="img/hermitcore_logo.png" />

# RustyHermit - A Rust-based, lightweight unikernel

[![Build Status](https://git.rwth-aachen.de/acs/public/hermitcore/rusty-hermit/badges/master/pipeline.svg)](https://git.rwth-aachen.de/acs/public/hermitcore/rusty-hermit/pipelines)
[![Build Status](https://travis-ci.com/hermitcore/rusty-hermit.svg?branch=master)](https://travis-ci.com/hermitcore/rusty-hermit)
![Actions Status](https://github.com/hermitcore/rusty-hermit/workflows/Test/badge.svg)
[![Slack Status](https://radiant-ridge-95061.herokuapp.com/badge.svg)](https://radiant-ridge-95061.herokuapp.com)

[RustyHermit](http://www.hermitcore.org) is a [unikernel](http://unikernel.org) targeting a scalable and predictable runtime for high-performance and cloud computing.
Unikernel means, you bundle your application directly with the kernel library, so that it can run without any installed operating system.
This reduces overhead, therefore, interesting applications include virtual machines and high-performance computing.

## Background

HermitCore was a research unikernel developed at [RWTH-Aachen](https://www.rwth-aachen.de) written in C ([libhermit](https://github.com/hermitcore/libhermit)).
**RustyHermit** is a rewrite of HermitCore written in [Rust](https://www.rust-lang.org).

The ownership  model of Rust guarantees memory/thread-safety and enables us to eliminate many classes of bugs at compile-time.
Consequently, the use of Rust for kernel development promises less vulnerabilities in comparison to common programming languages.

The kernel and the integration into the Rust runtime is entirely written in Rust and does not use any C/C++ Code.
We extend the Rust toolchain so that the build process is similar to Rust's usual workflow.
Rust applications that do not bypass the Rust runtime and directly use OS services are able to run on RustyHermit without modifications.

## Prerequisites

The Rust toolchain can be installed from the [official webpage](https://www.rust-lang.org/).
RusyHermit currently requires the **nightly versions** of the toolchain.
```sh
$ rustup default nightly
```

Further requirements are the source code of the Rust runtime,  [cargo-download](https://crates.io/crates/cargo-download), and llvm-tools:

```sh
$ cargo install cargo-download
$ rustup component add rust-src
$ rustup component add llvm-tools-preview
```

## Building RustyHermit

The repository contains following directories and submodules:

1. _demo_ is a small demo application based on the data-parallelism library [Rayon](https://github.com/rayon-rs/rayon)
2. _hermit-abi_ contains the platform APIs and builds the interface between library operating system and the application
3. _hermit-sys_ contains a crate to automate the build process of the library operating systems
4. _libhermit-rs_ is the kernel itself
5. _loader_ contains a loader to run RustyHermit on a common virtualization platforms ([Qemu](https://www.qemu.org)) or bare-mateal on a x86 system
6. _netbench_ provides some basic network benchmarks

To build RustyHermit, the repository and all submodules are required:

```sh
$ # Get our source code.
$ git clone https://github.com/hermitcore/rusty-hermit.git
$ cd rusty-hermit
$ git submodule init
$ git submodule update
```

The final step is building RustyHermit with all demo applications as follows:

```sh
$ cargo build -Z build-std=std,core,alloc,panic_abort --target x86_64-unknown-hermit
```

The resulting "hypervisor-ready" binaries then can be found in the directory `target/x86_64-unknown-hermit/debug`

### Controlling the number of kernel messages

RustyHermit uses the lightweight logging crate [log](https://github.com/rust-lang/log) to print kernel messages.
If the environment variable `HERMIT_LOG_LEVEL_FILTER` is set at compile time to a string matching the name of a [LevelFilter](https://docs.rs/log/0.4.8/log/enum.LevelFilter.html), then that value is used for the LevelFilter.
If the environment variable is not set, or the name doesn't match, then LevelFilter::Info is used by default, which is the same as it was before.

For instance, the following command build RustyHermit with debug messages:

```sh
$ HERMIT_LOG_LEVEL_FILTER=Debug cargo build -Z build-std=std,core,alloc,panic_abort --target x86_64-unknown-hermit
```

## Running RustyHermit

### Using uhyve as Hypervisor

RustyHermit can run within our own hypervisor [*uhyve*](https://github.com/hermitcore/uhyve) , which requires [KVM](https://www.linux-kvm.org/) to create a virtual machine.
Please install the hypervisor as follows:

```sh
cargo install uhyve
```

Afterwards, your are able to start RustyHermit applications within our hypervisor:

```sh
uhyve target/x86_64-unknown-hermit/debug/rusty_demo
```

More details can be found in the [uhyve README](https://github.com/hermitcore/uhyve).

### Using Qemu as Hypervisor

It is also possible to run RustyHermit within [Qemu](https://www.qemu.org).
RustyHermit produces 64-bit binaries, but Qemu's x86 emulation cannot boot them directly.
Therefore, the loader [rusty-loader](https://github.com/hermitcore/rusty-loader) is required to boot the application.
To build the loader, the assembler [nasm](https://www.nasm.us) is required.
After the installation, the loader can be build as follows.

```bash
$ git clone https://github.com/hermitcore/rusty-loader.git
$ cd rusty-loader
$ make
```

Afterwards, the loader is stored in `target/x86_64-unknown-hermit-loader/debug/` as `rusty-loader`.
As final step, the unikernel application `app` can be booted with following command:

```bash
$ qemu-system-x86_64 -display none -smp 1 -m 64M -serial stdio  -kernel path_to_loader/rusty-loader -initrd path_to_app/app -cpu qemu64,apic,fsgsbase,rdtscp,xsave,fxsr
```

It is important to enable the processor features _fsgsbase_ and _rdtscp_ because it is a prerequisite to boot RustyHermit.

You can provide arguments to the application via the kernel commandline, which you can set with qemu's `-append` option. Since both the kernel and the application can have parameters, they are separated with `--`:

```bash
qemu-system-x86_64 ... -append "kernel-arguments -- application-arguments"
```

## Building your own applications

To build own application based on RustyHermit, a new cargo project must be created:

```sh
cargo new hello_world
cd hello_world
```

To bind the library operating system to the application, add the crate [hermit-sys](https://crates.io/crates/hermit-sys) to the dependencies in the file *Cargo.toml*.
It is important to use at least the optimization level 1.
Consequently, it is required to **extend** *Cargo.toml* with following lines:

```toml
# Cargo.toml

[target.'cfg(target_os = "hermit")'.dependencies]
hermit-sys = "0.1.*"
default-features = false
features = ["smoltcp"]

[profile.release]
opt-level = 3

[profile.dev]
opt-level = 1
```

The feature `smoltcp` includes the network stack [smoltcp](https://github.com/smoltcp-rs/smoltcp) and offers the possibility of communication base on TCP/UDP.
To link the application with RustyHermit, declare `hermit_sys` an `external crate` in the main file of your application.

```rust
// src/main.rs

#[cfg(target_os = "hermit")]
extern crate hermit_sys;

fn main() {
        println!("Hello World!");
}
```

The final step is building the application as follows:

```sh
cargo build -Z build-std=std,core,alloc,panic_abort --target x86_64-unknown-hermit
```

The resulting "hypervisor-ready" binary then can be found in `target/x86_64-unknown-hermit/debug`

A simple example is published at [rusty-demo](https://github.com/hermitcore/rusty-demo).

To enable *Link Time Optimization* (LTO), please extend the release configuration in *Cargo.toml* as follows:

```toml
[profile.release]
opt-level = 3
lto = "thin"
```

In addition, the [Linker-plugin LTO](https://doc.rust-lang.org/rustc/linker-plugin-lto.html) have to be enabled by setting the compiler flag `linker-plugin-lto`.
In this case, the release version have to build as follows:

```sh
RUSTFLAGS="-Clinker-plugin-lto" cargo build -Z build-std=std,core,alloc,panic_abort --target x86_64-unknown-hermit --release
```

## Network support

To enable an ethernet device, we have to setup a tap device on the
host system. For instance, the following command establish the tap device
`tap10` on Linux:

```bash
$ sudo ip tuntap add tap10 mode tap
$ sudo ip addr add 10.0.5.1/24 broadcast 10.0.5.255 dev tap10
$ sudo ip link set dev tap10 up
$ sudo bash -c 'echo 1 > /proc/sys/net/ipv4/conf/tap100/proxy_arp'
```

Per default, RustyHermit's network interface uses `10.0.5.3` as IP address, `10.0.5.1`
for the gateway and `255.255.255.0` as network mask.
The default configuration could be overloaded at compile time by the environment variables
`HERMIT_IP`, `HERMIT_GATEWAY` and `HERMIT_MASK`.
For instance, the following command sets the IP address to `10.0.5.100`.

```sh
$ HERMIT_IP="10.0.5.100" cargo build -Z build-std=std,core,alloc,panic_abort --target x86_64-unknown-hermit
```

Currently, RustyHermit does only support network interfaces through [virtio](https://www.redhat.com/en/blog/introduction-virtio-networking-and-vhost-net).
To use it, you have to start RustyHermit in Qemu with following command:

```bash
$ qemu-system-x86_64 -cpu qemu64,apic,fsgsbase,rdtscp,xsave,fxsr \
        -enable-kvm -display none -smp 1 -m 1G -serial stdio \
        -kernel path_to_loader/rusty-loader \
        -initrd path_to_app/app \
        -netdev tap,id=net0,ifname=tap10,script=no,downscript=no,vhost=on \
        -device virtio-net-pci,netdev=net0,disable-legacy=on
```


## Using virtio-fs

The Kernel has rudimentary support for the virtio-fs shared file system. Currently only files, no folders are supported. To use it, you have to run a virtio-fs daemon and start qemu as described in [Standalone virtio-fs usage](https://virtio-fs.gitlab.io/howto-qemu.html):

```bash
# start virtiofsd in the background
$ sudo virtiofsd --thread-pool-size=1 --socket-path=/tmp/vhostqemu -o source=$(pwd)/SHARED_DIRECTORY
# give non-root-users access to the socket
$ sudo chmod 777 /tmp/vhostqemu
# start qemu with virtio-fs device.
# you might want to change the socket (/tmp/vhostqemu) and virtiofs tag (currently myfs)
$ qemu-system-x86_64 -cpu qemu64,apic,fsgsbase,rdtscp,xsave,fxsr -enable-kvm -display none -smp 1 -m 1G -serial stdio \
        -kernel path_to_loader/rusty-loader \
        -initrd path_to_app/app \
        -chardev socket,id=char0,path=/tmp/vhostqemu \
        -device vhost-user-fs-pci,queue-size=1024,chardev=char0,tag=myfs \
        -object memory-backend-file,id=mem,size=1G,mem-path=/dev/shm,share=on \
        -numa node,memdev=mem
```

You can now access the files in SHARED_DIRECTORY under the virtiofs tag like `/myfs/testfile`.


## Use RustyHermit for C/C++, Go, and Fortran applications

If you are interested to build C/C++, Go, and Fortran applications on top of a Rust-based library operating systen, please take a look at [https://github.com/hermitcore/hermit-playground](https://github.com/hermitcore/hermit-playground).


## Missing features

* Multikernel support (might be comming)
* Virtio support (partly available)
* Network support (partly available)

## Troubleshooting

### command failed with the error message `linker `rust-lld` not found`

The path to the *llvm-tools* is not set.
On Linux, it is typically installed at *${HOME}/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin*.
```sh
PATH=${HOME}/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin:$PATH cargo build -Z build-std=std,core,alloc,panic_abort --target x86_64-unknown-hermit
```
Otherwise, the linker can be replaced by *lld* as follows:

```sh
RUSTFLAGS="-C linker=lld" cargo build -Z build-std=std,core,alloc,panic_abort --target x86_64-unknown-hermit
```


## Credits

RustyHermit is derived from following tutorials and software distributions:

1. Philipp Oppermann's [excellent series of blog posts][opp].
2. Erik Kidd's [toyos-rs][kidd], which is an extension of Philipp Opermann's kernel.
3. The Rust-based teaching operating system [eduOS-rs][eduos].

[opp]: http://blog.phil-opp.com/
[kidd]: http://www.randomhacks.net/bare-metal-rust/
[eduos]: http://rwth-os.github.io/eduOS-rs/

HermitCore's Emoji is provided for free by [EmojiOne](https://www.gfxmag.com/crab-emoji-vector-icon/).


## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.


## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

RustyHermit is being developed on [GitHub](https://github.com/hermitcore/rusty-hermit).
Create your own fork, send us a pull request, and chat with us on [Slack](https://radiant-ridge-95061.herokuapp.com)
