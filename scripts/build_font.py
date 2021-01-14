import subprocess as sp
import re

# fun!
font = [
    int(hex, 16)
    for hex in [
        val[:-1]
        for val in [
            s.strip()
            for s in re.sub(
                r"/\*.*\*/",
                "",
                "\n".join(
                    sp.getoutput("psf2bsd res/zap-light16.psf")
                    .split("/* 0x20  (' ') */")[1]
                    .split("\n")[:-1]
                ).strip(),
            ).split("\n")
            if s.strip() != ""
        ]
        if val[-1] == ","
    ]
]

print(
    """#![deny(missing_docs)]
#![no_std]

//! zap_font provides access to the 8x16 Zap font as a byte array.

/// The byte array containing the font.
pub static FONT: &[u8] = &["""
)
for b in font:
    print("\t0b{:08b},".format(b))
print("];")
