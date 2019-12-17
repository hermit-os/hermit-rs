FROM ubuntu:latest

ENV DEBIAN_FRONTEND=noninteractive

# Update Software repository
RUN apt-get clean 
RUN apt-get -qq update

# Install required packets from ubuntu repository
RUN apt-get install -y apt-transport-https curl wget vim nano git binutils autoconf automake make cmake qemu-kvm qemu-system-x86 nasm gcc g++ build-essential libtool bsdmainutils pkg-config libssl-dev

# Install Rust toolchain
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly
RUN /root/.cargo/bin/cargo install cargo-download
RUN /root/.cargo/bin/cargo install uhyve
RUN /root/.cargo/bin/rustup component add rust-src
RUN /root/.cargo/bin/rustup component add llvm-tools-preview

ENV PATH="/root/.cargo/bin:${PATH}"
ENV EDITOR=vim

# Switch back to dialog for any ad-hoc use of apt-get
ENV DEBIAN_FRONTEND=dialog

