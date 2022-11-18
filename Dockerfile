# Derived from rust:bullseye
FROM buildpack-deps:bullseye as hermit-toolchain

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    # Manually sync this with rust-toolchain.toml!
    RUST_VERSION=nightly-2022-10-19 \
    RUST_COMPONENTS="clippy llvm-tools-preview rustfmt rust-src"

RUN set -eux; \
    dpkgArch="$(dpkg --print-architecture)"; \
    case "${dpkgArch##*-}" in \
        amd64) rustArch='x86_64-unknown-linux-gnu'; rustupSha256='5cc9ffd1026e82e7fb2eec2121ad71f4b0f044e88bca39207b3f6b769aaa799c' ;; \
        armhf) rustArch='armv7-unknown-linux-gnueabihf'; rustupSha256='48c5ecfd1409da93164af20cf4ac2c6f00688b15eb6ba65047f654060c844d85' ;; \
        arm64) rustArch='aarch64-unknown-linux-gnu'; rustupSha256='e189948e396d47254103a49c987e7fb0e5dd8e34b200aa4481ecc4b8e41fb929' ;; \
        i386) rustArch='i686-unknown-linux-gnu'; rustupSha256='0e0be29c560ad958ba52fcf06b3ea04435cb3cd674fbe11ce7d954093b9504fd' ;; \
        *) echo >&2 "unsupported architecture: ${dpkgArch}"; exit 1 ;; \
    esac; \
    url="https://static.rust-lang.org/rustup/archive/1.25.1/${rustArch}/rustup-init"; \
    wget "$url"; \
    echo "${rustupSha256} *rustup-init" | sha256sum -c -; \
    chmod +x rustup-init; \
    ./rustup-init -y --no-modify-path \
        --profile minimal \
        --default-toolchain $RUST_VERSION \
        --default-host ${rustArch} \
        --component $RUST_COMPONENTS \
    ; \
    rm rustup-init; \
    chmod -R a+w $RUSTUP_HOME $CARGO_HOME; \
    rustup --version; \
    cargo --version; \
    rustc --version;

# Build dependencies with stable toolchain channel
FROM rust:bullseye as stable-deps
RUN set -eux; \
    cargo install --locked uhyve;

# Install dependencies
FROM hermit-toolchain as ci-runner
RUN set -eux; \
	apt-get update; \
	apt-get install -y --no-install-recommends \
        # Build dependencies
        # For SMP:
        nasm \
        \
        # Test dependencies
        qemu-system-x86 \
        # For qemu-img:
        qemu-utils \
        # For kvm-ok:
		cpu-checker \
        # For sfdisk:
        fdisk \
        kpartx \
        # For grub-install:
        grub2-common \
        # For GRUB modules:
        grub-pc-bin \
    ; \
	rm -rf /var/lib/apt/lists/*;
COPY --from=stable-deps $CARGO_HOME/bin/uhyve $CARGO_HOME/bin/uhyve
