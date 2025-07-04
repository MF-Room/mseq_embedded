CHIP := --chip STM32F411CEUx
GDB ?= arm-none-eabi-gdb
SIZE ?= arm-none-eabi-size
PACKAGE := -p kernel

flash:
	cargo flash $(CHIP) $(PACKAGE) -- -r

rtt:
	cargo run $(PACKAGE) -r

build:
	cargo build $(PACKAGE) -r

gdb_server:
	$(MAKE) flash_debug
	probe-rs gdb $(CHIP)

gdb:
	 $(GDB) -x init.gdb target/thumbv7em-none-eabihf/debug/kernel

flash_debug:
	cargo flash $(CHIP) $(PACKAGE)

size:
	cargo build $(PACKAGE) -r
	$(SIZE) -G target/thumbv7em-none-eabihf/release/kernel
