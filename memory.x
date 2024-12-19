MEMORY {
    BOOT2 : ORIGIN = 0x10000000, LENGTH = 0x200
    FLASH : ORIGIN = 0x10000200, LENGTH = 2048K - 0x100
    RAM   : ORIGIN = 0x20000000, LENGTH = 256K

    /** From the pico-sdk memory mapping */
    /** RAM(rwx) : ORIGIN =  0x20000000, LENGTH = 256k */
    SCRATCH_X(rwx) : ORIGIN = 0x20060000, LENGTH = 4k
    SCRATCH_Y(rwx) : ORIGIN = 0x20061000, LENGTH = 4k
}

EXTERN(BOOT2_FIRMWARE)

SECTIONS {
    /* ### Boot loader */
    .boot2 ORIGIN(BOOT2) :
    {
        KEEP(*(.boot2));
    } > BOOT2
} INSERT BEFORE .text;
