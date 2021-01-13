#!/usr/bin/python3

import sys
import os

commands = []
verbose = False

_ = lambda c: commands.append(c)


def vprint(message):
    if verbose:
        print(message)


def clean():
    _("rm -f boot.img")


def build():
    _('RUSTFLAGS="-C link-arg=-Ttools/linker.ld" cargo build')
    _("cp target/x86_64-bruh_os/debug/bruh_os build/kernel.elf")
    _("dd if=/dev/zero of=build/bruhos.img bs=1M count=64")
    _("mkfs.ext2 build/bruhos.img")
    _("sudo mount build/bruhos.img iso/")
    _("sudo cp -Rf build/* iso/")
    _("sync")
    _("sudo umount iso/")
    _("./lib/limine/limine-install build/bruhos.img")


def run():
    _(
        "qemu-system-x86_64 -m 2G -net none -smp 4 -drive format=raw,file=build/bruhos.img"
    )


def all():
    clean()
    build()
    run()


def help():
    print(
        """x.py - BruhOS v0.1
USAGE: ./x.py [-v] subcommand

SUBCOMMANDS:
    clean    - clean out build files
    build    - build kernel and write to hard disk
    run      - emulate with qemu
    all      - clean, build, and run
    help     - display this message"""
    )


def main():
    actions = {"all": all, "clean": clean, "build": build, "run": run, "help": help}

    if "-v" in sys.argv:
        verbose = True
        sys.argv.remove("-v")

    action = sys.argv.pop()

    if action not in actions:
        print("Invalid action!\n")
        help()
        exit(1)

    actions[action]()
    for command in commands:
        vprint(f"[ INFO ] => Running: {command}")
        os.system(command)

    vprint(f"[ INFO ] => Complete!")


if __name__ == "__main__":
    main()
