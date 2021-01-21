# BruhOS

a basic x86_64 kernel in the works.

## cool stuff
- written in rust
- boots with any stivale2-compliant bootloader
- framebuffer bitmap font renderer
- pmm (bitmap allocator)

## deps

- binutils
- qemu
- qemu-system-x86
- rustup (for nightly)
  - lld
- cc (for limine)
- git
- python3
- mkfs
- make
- parted
- psf2bsd (to build font)

## build

```
x.py - BruhOS v0.1
USAGE: ./x.py [-v] subcommand

SUBCOMMANDS:
    clean    - clean out build files
    build    - build kernel
    hdd      - create and write to hard disk
    run      - emulate with qemu
    all      - clean, build, and run
    help     - display this message
```
