# An example program to test the NVMe driver

> Note: PCI devices are not yet implemented for `riscv64`.  

Complete following steps to run the test with QEMU:

- Install [QEMU](https://www.qemu.org) to get the commands
`qemu-system-x86_64`, `qemu-system-aarch64`.

- Download the [hermit-loaders](https://github.com/hermit-os/loader/releases) for
`x86_64`, `aarch64` and place them into this nvme-test directory.

> `riscv64` only: download [OpenSBI](https://github.com/riscv-software-src/opensbi/releases),
rename the directory to `opensbi` and place it into this nvme-test directory.

- Execute `make run-x86_64` (default) or `make run-aarch64` to run the program.

- Cleanup files with `make clean` afterwards.

