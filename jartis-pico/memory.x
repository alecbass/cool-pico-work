MEMORY {
    BOOT2 : ORIGIN = 0x10000000, LENGTH = 0x100
    FLASH : ORIGIN = 0x10000100, LENGTH = 2048K - 0x100
    RAM   : ORIGIN = 0x20000000, LENGTH = 256K
    SCRATCH_X(rwx) : ORIGIN = 0x20040000, LENGTH = 4k
    SCRATCH_Y(rwx) : ORIGIN = 0x20041000, LENGTH = 4k
}

EXTERN(BOOT2_FIRMWARE)

SECTIONS {
    /* ### Boot loader */
    /*
    NOTE: This commented stuff can be removed if embassy-rp remains as a dependency, as it provides the same .boot2 symbol
    .boot2 : {
        __boot2_start__ = .;
        KEEP (*(.boot2))
        __boot2_end__ = .;
    } > BOOT2

    ASSERT(__boot2_end__ - __boot2_start__ <= 0x100,
        "ERROR: Pico second stage bootloader must be within 256 bytes in size")
    */
} INSERT BEFORE .text;
