# Hermit for Rust

[![Crates.io](https://img.shields.io/crates/v/hermit.svg)](https://crates.io/crates/hermit)
[![docs.rs](https://img.shields.io/docsrs/hermit)](https://docs.rs/hermit)

This crate builds and links against the [Hermit kernel](https://github.com/hermit-os/kernel) to create a Hermit unikernel image.

This crate is no longer distributed via crates.io.
To upgrade, use the crate via Git instead:

```diff
-hermit = "0.12"
+hermit = { git = "https://github.com/hermit-os/hermit-rs.git", tag = "hermit-0.13.0" }
```

For details, see [hermit-os/hermit-rs#876](https://github.com/hermit-os/hermit-rs/pull/876).
