<img width="100" align="right" src="img/hermitcore_logo.png" />

# RustyHermit - A Rust-based, lightweight unikernel

[![Zulip Badge](https://img.shields.io/badge/chat-hermit-57A37C?logo=zulip)](https://hermit.zulipchat.com/)

[RustyHermit](http://www.hermitcore.org) is a [unikernel](http://unikernel.org) targeting a scalable and predictable runtime for high-performance and cloud computing.
Unikernel means, you bundle your application directly with the kernel library, so that it can run without any installed operating system.
This reduces overhead, therefore, interesting applications include virtual machines and high-performance computing.

The kernel is able to run [Rust](https://github.com/hermitcore/rusty-hermit) applications, as well as [C/C++/Go/Fortran](https://github.com/hermitcore/hermit-playground) applications.

The repository contains following directories and submodules:

1. _demo_ is a small demo application based on the data-parallelism library [Rayon](https://github.com/rayon-rs/rayon)
2. _hermit-abi_ contains the platform APIs and builds the interface between library operating system and the application
3. _hermit_ contains a crate to automate the build process of the library operating systems
4. _kernel_ is the kernel itself
5. _netbench_ provides some basic network benchmarks

## Background

**RustyHermit** is a rewrite of HermitCore in [Rust](https://www.rust-lang.org) developed at [RWTH-Aachen](https://www.rwth-aachen.de).
HermitCore was a research unikernel written in C ([libhermit](https://github.com/hermitcore/libhermit)).

The ownership  model of Rust guarantees memory/thread-safety and enables us to eliminate many classes of bugs at compile-time.
Consequently, the use of Rust for kernel development promises fewer vulnerabilities in comparison to common programming languages.

The kernel and the integration into the Rust runtime are entirely written in Rust and do not use any C/C++ Code.
We extended the Rust toolchain so that the build process is similar to Rust's usual workflow.
Rust applications that use the Rust runtime and do not directly use OS services are able to run on RustyHermit without modifications.

## Requirements

* [`rustup`](https://www.rust-lang.org/tools/install)
* [NASM](https://nasm.us/) (only for SMP on x86_64)

## Building your own applications

Have a look at [rusty-demo](https://github.com/hermitcore/rusty-demo).


## Use RustyHermit for C/C++, Go, and Fortran applications

If you are interested to build C/C++, Go, and Fortran applications on top of a Rust-based library operating system, please take a look at [https://github.com/hermitcore/hermit-playground](https://github.com/hermitcore/hermit-playground).

## Missing features

* Multikernel support (might be coming)
* Virtio support (partly available)
* Network support (partly available)

## Wiki

Please use the [Wiki](https://github.com/hermitcore/rusty-hermit/wiki) to get further information and configuration options.

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
Create your own fork, send us a pull request, and chat with us on [Zulip](https://hermit.zulipchat.com/).

## Publications

The following scientific publications are related to RustyHermit:

- S. Lankes et al. - [Exploring Rust for Unikernel Development](https://dl.acm.org/doi/10.1145/3365137.3365395)
- M. Sung et al. - [Intra-unikernel isolation with Intel memory protection keys](https://dl.acm.org/doi/10.1145/3381052.3381326)
- S. Lankes et al. - [RustyHermit: A Scalable, Rust-Based Virtual Execution Environment](https://link.springer.com/chapter/10.1007/978-3-030-59851-8_22)

## Funding

The development of this project was partially funded by the European Union’s Horizon 2020 research and innovation programme under grant agreement No 957246 - IoT-NGIN.
