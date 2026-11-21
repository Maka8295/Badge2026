# Badge programs

The badge can run native programs from the SD card. A program is an ESP-IDF
project built as a loadable ELF file (`<name>.app.elf`). In the Files app,
select a file ending in `.elf` and press A to run it. When the program returns
from `app_main`, the badge goes back to the file list.

Programs run with no sandbox: they use the same CPU, RAM and task as the
firmware. A crash in a program reboots the badge.

## API

`badge.h` in this folder is the full API: drawing, keys, time and the badge
name. Programs can also call the libc and ESP-IDF functions that the loader
exports (see `esp_all_symbol.c` in the `espressif/elf_loader` component).
Any other undefined symbol makes loading fail with `unknown symbol (-88)`;
the serial log names the missing symbol.

## Build

Start from `apps/hello`:

- `CMakeLists.txt` calls `include(elf_loader)` and `project_elf(<name>)` after `project(<name>)`.
- `main/idf_component.yml` depends on `espressif/elf_loader` 1.3.3.
- `main/CMakeLists.txt` adds `sdk/` to the include path.
- `sdkconfig.defaults` sets the target to `esp32s3`.

Build it from the `firmware` folder:

```
make app APP=hello
```

This prints the path of `apps/hello/build/hello.app.elf`. The first build
compiles ESP-IDF once and takes a few minutes.

## Run

Copy the `.app.elf` file anywhere on the SD card (FAT), for example:

```
cp apps/hello/build/hello.app.elf /media/$USER/SDCARD/hello.elf
```

Put the card in the badge, open Files, select `hello.elf` and press A.

## Limits

- The ELF file and the loaded program are both kept in RAM. The badge has 8 MB of PSRAM,
  and programs are loaded into it, so a few MB is fine. The badge refuses files that clearly do not fit.
- `malloc` of more than 16 KB comes from PSRAM, which is slower than internal RAM.
- The program runs on the main task stack, so keep large buffers off the stack
  (use `malloc`).
- Call `badge_delay_ms` in every loop so the rest of the system can run.
