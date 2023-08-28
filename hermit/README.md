# Hermit for Rust

[![Crates.io](https://img.shields.io/crates/v/hermit.svg)](https://crates.io/crates/hermit)
[![docs.rs](https://img.shields.io/docsrs/hermit)](https://docs.rs/hermit)

This crate builds and links against the [Hermit kernel](https://github.com/hermitcore/kernel) to create a Hermit unikernel image.

## Building

1.  Add the following to your `Cargo.toml`:

    ```toml
    [target.'cfg(target_os = "hermit")'.dependencies]
    hermit = "0.6"
    ```

2.  Add the following to your `main.rs`:

    ```rust
    #[cfg(target_os = "hermit")]
    use hermit as _;
    ```

3.  Build against one of the [`*-unknown-hermit`] targets.

    [`*-unknown-hermit`]: https://doc.rust-lang.org/nightly/rustc/platform-support/hermit.html

    Either

    -   install [rust-std-hermit] on stable Rust

        [rust-std-hermit]: https://github.com/hermitcore/rust-std-hermit

    or

    -   use `-Zbuild-std=std,panic_abort` on nightly Rust.

## Running

You can boot the resulting image in the specialized [Uhyve] unikernel hypervisor or on other platforms like QEMU using the [Hermit loader].

[Uhyve]: https://github.com/hermitcore/uhyve
[Hermit loader]: https://github.com/hermitcore/loader

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
