FROM ubuntu:latest

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && \
    apt-get -y install cpu-checker util-linux apt-transport-https curl wget binutils build-essential gcc libtool bsdmainutils pkg-config libssl-dev git qemu-kvm qemu-system-x86 nasm seabios qemu-utils fdisk grub-pc grub-pc-bin grub-imageboot grub-legacy-ec2 multiboot kpartx gzip && \
    apt-get clean

# Install Rust toolchain
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly --profile minimal
RUN /root/.cargo/bin/cargo install cargo-download

ENV PATH="/root/.cargo/bin:/root/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/:${PATH}"
ENV EDITOR=vim

# Switch back to dialog for any ad-hoc use of apt-get
ENV DEBIAN_FRONTEND=dialog

