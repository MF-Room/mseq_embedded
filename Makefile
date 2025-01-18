CHIP := STM32F411CEUx
GDB ?= arm-none-eabi-gdb
SIZE ?= arm-none-eabi-size

flash_debug:
	cargo flash --bin minimal

flash:
	cargo flash --bin minimal --release

rtt:
	cargo rb minimal -r

gdb_server:
	$(MAKE) flash_debug
	probe-rs gdb --chip $(CHIP)

gdb:
	 $(GDB) -x init.gdb target/thumbv7em-none-eabihf/debug/mseq_hardware

size:
	cargo build --release
	$(SIZE) -G target/thumbv7em-none-eabihf/release/mseq_hardware
