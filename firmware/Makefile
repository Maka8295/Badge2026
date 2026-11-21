SHELL := /bin/bash
PORT ?= $(firstword $(wildcard /dev/ttyACM*))
CHIP := esp32s3
ESPTOOL := uv run esptool --chip $(CHIP) -p $(PORT)
TS := $(shell date +%Y%m%d-%H%M%S)
RS := cd rs && . $(HOME)/export-esp.sh &&
IDF := . $(HOME)/esp/esp-idf/export.sh > /dev/null &&
APP ?= hello
APP_OFFSET := 0x10000

# encrypts hunt/hunt.toml into rs/hunt.bin (builds the crackme app first)
hunt:
	uv run python hunt/build_hunt.py crackme
	$(MAKE) app APP=crackme
	uv run python hunt/build_hunt.py pack

rs-build: hunt
	$(RS) cargo build --release

# make app APP=hello  (builds apps/hello into apps/hello/build/hello.app.elf)
app:
	cd apps/$(APP) && $(IDF) idf.py elf
	@echo "app: $(CURDIR)/apps/$(APP)/build/$(APP).app.elf"

release: rs-build app
	mkdir -p release
	espflash save-image --chip $(CHIP) --merge --skip-padding -s 8mb rs/target/xtensa-esp32s3-espidf/release/badge release/badge2026.bin
	cp apps/hello/build/hello.app.elf release/hello.elf
	sha256sum release/*

rs-flash:
	$(RS) espflash flash --monitor -p $(PORT) target/xtensa-esp32s3-espidf/release/badge

# serial log with panic backtraces decoded against the ELF
rs-monitor:
	$(RS) espflash monitor -p $(PORT) --elf target/xtensa-esp32s3-espidf/release/badge

info:
	$(ESPTOOL) chip-id
	$(ESPTOOL) flash-id
	uv run espefuse --chip $(CHIP) -p $(PORT) summary

# full flash backup, size auto-detected
dump:
	mkdir -p dumps
	$(ESPTOOL) -b 921600 read-flash 0 ALL dumps/flash-$(TS).bin
	sha256sum dumps/flash-$(TS).bin

# make flash BIN=dumps/flash-xxx.bin  (writes at 0x0)
flash:
	test -n "$(BIN)"
	$(ESPTOOL) -b 921600 write-flash 0x0 $(BIN)

# writes only the app from release/badge2026.bin; keeps NVS (hunt progress, chat settings)
flash-app:
	tail -c +$$(( $(APP_OFFSET) + 1 )) release/badge2026.bin > release/app.bin
	$(ESPTOOL) -b 921600 write-flash $(APP_OFFSET) release/app.bin
	rm release/app.bin

erase:
	$(ESPTOOL) erase-flash

monitor:
	uv run python -m serial.tools.miniterm --raw $(PORT) 115200

.PHONY: hunt app rs-build rs-flash rs-monitor info dump flash flash-app erase monitor
