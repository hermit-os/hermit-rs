SOURCE_FILES := $(shell test -e src/ && find src -type f)

.PHONY: build
build: target/x86_64-unknown-hermit/release/wasmi_demo wasm/fib.wasm

target/x86_64-unknown-hermit/release/wasmi_demo: $(SOURCE_FILES) Cargo.* wasm/fib.wasm
	cargo build \
		-Zbuild-std=std,panic_abort \
		--target x86_64-unknown-hermit \
		--release

wasm/fib.wasm:
	cd examples; cd fib; cargo build --target wasm32-wasi --release; cp target/wasm32-wasi/release/fib.wasm ../../wasm
	
.PHONY: clean
clean:
	cargo clean
	rm -f wasm/fib.wasm

.PHONY: run
run: target/x86_64-unknown-hermit/release/wasmi_demo
	qemu-system-x86_64 \
		-cpu qemu64,apic,fsgsbase,fxsr,rdrand,rdtscp,xsave,xsaveopt \
		-display none -serial stdio \
		-smp 4 \
		-m 1G \
		-device isa-debug-exit,iobase=0xf4,iosize=0x04 \
		-kernel hermit-loader-x86_64 \
		-initrd target/x86_64-unknown-hermit/release/wasmi-demo \
		-netdev user,id=u1,hostfwd=tcp::3000-:3000 -device virtio-net-pci,netdev=u1,disable-legacy=on,packed=on,mq=on
