CHIP := STM32F411CEUx
GDB ?= arm-none-eabi-gdb

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
