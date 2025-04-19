# MSeq Embedded

## Targeted Microcontroller
* STM32F411CE

## Development Tools
* probe-rs
* Arm GNU Toolchain (arm-none-eabi) (gdb)
* rustup target add thumbv7em-none-eabihf
* cargo install flip-link

## Usage

Midi UART:
* RX: A10
* TX: A9

### Flash only

```bash
make flash
```

### Flash and use RTT

```bash
make rtt
```

### Debug

Open GDB server:
```bash
make gdb_server
```
Open GDB client:
```bash
make gdb
```
