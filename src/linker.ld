/* kmain as entrypoint */
ENTRY(kmain)

SECTIONS
{
    /* We wanna be placed in the higher half, 2MiB above 0 in physical memory. */
    . = 0xffffffff80200000;

    /* give boot header its own (early) section */
    .stivale2hdr : {
        KEEP(*(.stivale2hdr))
    }

    /* misc elf sections */
    .text : {
        *(.text*)
    }

    .rodata : {
        *(.rodata*)
    }

    .data : {
        *(.data*)
    }

    .bss : {
        *(COMMON)
        *(.bss*)
    }
}
