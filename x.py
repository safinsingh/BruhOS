#!/usr/bin/python3

import sys
import os
from subprocess import call, PIPE, DEVNULL

commands = []
quiet = False

_ = lambda c: commands.append(c)


def iprint(message):
    print("\033[1m\033[36m[ info ] =>\033[0m " + message)


def eprint(message):
    print("\033[1m\033[31m[ fail ] =>\033[0m " + message)
    exit(1)


def sprint(message):
    print("\033[1m\033[32m[ scss ] =>\033[0m " + message)


def clean():
    _("rm -f build/*")
    _("sudo umount -R isotmp/ | cat")
    _("sudo rm -rf isotmp/")
    _("rm -f loopback_dev")


def build():
    _('RUSTFLAGS="-C link-arg=-Tsrc/linker.ld" cargo build')
    _("cp target/x86_64-bruh_os/debug/bruh_os build/kernel.elf")


def hdd():
    _("dd if=/dev/zero of=build/bruhos.img bs=1M count=64")
    _("parted -s build/bruhos.img mklabel gpt")
    _("parted -s build/bruhos.img mkpart primary 2048s 100%")
    _("sudo losetup -Pf --show build/bruhos.img >loopback_dev")
    _("mkdir -p isotmp")
    _("sudo partprobe $(cat loopback_dev)")
    _("sudo mkfs.ext2 $(cat loopback_dev)p1")
    _("sudo mount $(cat loopback_dev)p1 isotmp/")
    _("sudo cp -Rf build/kernel.elf isotmp/")
    _("sudo cp -Rf run/limine.cfg isotmp/")
    _("sync")
    _("sudo umount isotmp/")
    _("sudo losetup -d $(cat loopback_dev)")
    _("./lib/limine/limine-install build/bruhos.img")


def run():
    _(
        "qemu-system-x86_64 -m 8G -net none -smp 4 -drive format=raw,file=build/bruhos.img"
    )


def all():
    clean()
    build()
    hdd()
    run()
    clean()


def help():
    print(
        """x.py - BruhOS v0.1
USAGE: ./x.py [-q] subcommand

FLAGS:
    -q       - run quietly

SUBCOMMANDS:
    clean    - clean out build files
    build    - build kernel
    hdd      - create and write to hard disk
    run      - emulate with qemu
    all      - clean, build, and run
    help     - display this message"""
    )


def main():
    actions = {
        "all": all,
        "clean": clean,
        "build": build,
        "run": run,
        "help": help,
        "hdd": hdd,
    }

    if "-q" in sys.argv:
        quiet = True
        sys.argv.remove("-q")

    action = sys.argv.pop()

    if action not in actions:
        print("Invalid action!\n")
        help()
        exit(1)

    actions[action]()
    for command in commands:
        if not quiet:
            iprint(f"Running: {command}")

        if call(["sh", "-c", command], stdout=DEVNULL, stderr=PIPE) != 0:
            eprint(f"Failed on: {command}")

    sprint("Complete!")


if __name__ == "__main__":
    main()
