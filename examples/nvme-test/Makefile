.PHONY: run-x86_64
run-x86_64: build-x86_64 disk-image
	# https://github.com/hermit-os/loader?tab=readme-ov-file#x86-64
	qemu-system-x86_64 \
		-cpu qemu64,apic,fsgsbase,fxsr,rdrand,rdtscp,xsave,xsaveopt \
		-smp 1 \
		-m 128M \
		-device isa-debug-exit,iobase=0xf4,iosize=0x04 \
		-display none \
		-serial stdio \
		-kernel hermit-loader-x86_64 \
		-initrd ../../target/x86_64-unknown-hermit/release/nvme-test \
		-drive file=nvme.img,format=raw,if=none,id=nvm \
		-device nvme,serial=deadbeef,drive=nvm

.PHONY: run-aarch64
run-aarch64: build-aarch64 disk-image
	# https://github.com/hermit-os/loader?tab=readme-ov-file#aarch64
	qemu-system-aarch64 \
		-machine virt,gic-version=3 \
		-cpu cortex-a76 \
		-smp 1 \
		-m 512M \
		-semihosting \
		-display none \
		-serial stdio \
		-kernel hermit-loader-aarch64 \
		-device guest-loader,addr=0x48000000,initrd=../../target/aarch64-unknown-hermit/release/nvme-test \
		-drive file=nvme.img,format=raw,if=none,id=nvm \
		-device nvme,serial=deadbeef,drive=nvm

# .PHONY: run-riscv64
# run-riscv64: build-riscv64 disk-image
# 	# https://github.com/hermit-os/loader?tab=readme-ov-file#64-bit-risc-v
# 	qemu-system-riscv64 \
# 		-machine virt \
# 		-cpu rv64 \
# 		-smp 1 \
# 		-m 128M \
# 		-display none \
# 		-serial stdio \
# 		-bios opensbi/share/opensbi/lp64/generic/firmware/fw_jump.bin \
# 		-kernel hermit-loader-riscv64 \
# 		-initrd ../../target/riscv64gc-unknown-hermit/release/nvme-test \
# 		-drive file=nvme.img,format=raw,if=none,id=nvm \
# 		-device nvme,serial=deadbeef,drive=nvm

.PHONY: build-x86_64
build-x86_64: 
	HERMIT_LOG_LEVEL_FILTER=ERROR \
	cargo build -Z build-std=std,core,alloc,panic_abort \
	--target x86_64-unknown-hermit --release

.PHONY: build-aarch64
build-aarch64: 
	HERMIT_LOG_LEVEL_FILTER=ERROR \
	cargo build -Z build-std=std,core,alloc,panic_abort \
	--target aarch64-unknown-hermit --release

# .PHONY: build-riscv64
# build-riscv64: 
# 	HERMIT_LOG_LEVEL_FILTER=ERROR \
# 	cargo build -Z build-std=std,core,alloc,panic_abort \
# 	--target riscv64gc-unknown-hermit

.PHONY: disk-image
disk-image:
	qemu-img create nvme.img 4M
	
.PHONY: clean
clean:
	cargo clean
	rm -f nvme.img
