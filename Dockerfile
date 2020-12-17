FROM ubuntu:latest

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && \
    apt-get -y install cpu-checker apt-transport-https curl wget binutils build-essential gcc

# Install Rust toolchain
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly
RUN /root/.cargo/bin/cargo install cargo-download
RUN /root/.cargo/bin/cargo install uhyve
RUN /root/.cargo/bin/rustup component add rust-src
RUN /root/.cargo/bin/rustup component add llvm-tools-preview

ENV PATH="/root/.cargo/bin:/root/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/:${PATH}"
ENV EDITOR=vim

# Switch back to dialog for any ad-hoc use of apt-get
ENV DEBIAN_FRONTEND=dialog

