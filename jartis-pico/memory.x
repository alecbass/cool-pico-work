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

    PROVIDE(__logical_binary_start = LOADADDR(.text));
    PROVIDE(__binary_info_header_end = __etext);

    /* ## Sections in RAM */
    /* ### .data */
    .c_data : ALIGN(4)
    {
        __data_start__ = .;
        // __data_start__ = LOADADDR(.data);
        // __data_end__ = SIZEOF(.data);
        *(vtable)

        *(.time_critical*)

        . = ALIGN(4);
        *(.after_data.*)
        . = ALIGN(4);
        /* preinit data */
        PROVIDE_HIDDEN (__mutex_array_start = .);
        KEEP(*(SORT(.mutex_array.*)))
        KEEP(*(.mutex_array))
        PROVIDE_HIDDEN (__mutex_array_end = .);

        . = ALIGN(4);
        *(.jcr)

        . = ALIGN(4); /* 4-byte align the end (VMA) of this section */
        __data_end__ = .;
    } > RAM
    /* PROVIDE(__data_start__ = __sdata); */
    /* PROVIDE(__data_end__ = __edata); */

    .tbss (NOLOAD) : {
        . = ALIGN(4);
        // __bss_start__ = .;
        __tls_base = .;
        *(.tbss .tbss.* .gnu.linkonce.tb.*)
        *(.tcommon)

        __tls_end = .;
    } > RAM
    PROVIDE(__bss_start__ = __sbss);
    PROVIDE(__bss_end__ = __ebss);

    /** Checked up to here before it starts failing */

    .c_bss (NOLOAD) : {
        . = ALIGN(4);
        __tbss_end = .;

        *(SORT_BY_ALIGNMENT(SORT_BY_NAME(.c_bss*)))
        *(COMMON)
        . = ALIGN(4);
        // __bss_end__ = .;
    } > RAM

    .c_heap (NOLOAD):
    {
        __end__ = __sheap;
        end = __end__;
        KEEP(*(.c_heap*))
        __HeapLimit = .;
    } > RAM

    /* Start and end symbols must be word-aligned */
    .scratch_x : {
        __scratch_x_start__ = .;
        *(.scratch_x.*)
        . = ALIGN(4);
        __scratch_x_end__ = .;
    } > SCRATCH_X AT > FLASH
    __scratch_x_source__ = LOADADDR(.scratch_x);

    .scratch_y : {
        __scratch_y_start__ = .;
        *(.scratch_y.*)
        . = ALIGN(4);
        __scratch_y_end__ = .;
    } > SCRATCH_Y AT > FLASH
    __scratch_y_source__ = LOADADDR(.scratch_y);

    .flash_end : {
        KEEP(*(.embedded_end_block*))
        PROVIDE(__flash_binary_end = .);
    } > FLASH

    __StackLimit = ORIGIN(RAM) + LENGTH(RAM);
    __StackOneTop = ORIGIN(SCRATCH_X) + LENGTH(SCRATCH_X);
    __StackTop = ORIGIN(SCRATCH_Y) + LENGTH(SCRATCH_Y);
    __StackOneBottom = __StackOneTop - SIZEOF(.stack1_dummy);
    __StackBottom = __StackTop - SIZEOF(.stack_dummy);
    PROVIDE(__stack = __StackTop);

    /* picolibc and LLVM */
    PROVIDE (__heap_start = __end__);
    PROVIDE (__heap_end = __HeapLimit);
    PROVIDE( __tls_align = MAX(ALIGNOF(.tdata), ALIGNOF(.tbss)) );
    PROVIDE( __tls_size_align = (__tls_size + __tls_align - 1) & ~(__tls_align - 1));
    PROVIDE( __arm32_tls_tcb_offset = MAX(8, __tls_align) );

    /* llvm-libc */
    PROVIDE (_end = __end__);
    PROVIDE (__llvm_libc_heap_limit = __HeapLimit);

    /* Check if data + heap + stack exceeds RAM limit */
    ASSERT(__StackLimit >= __HeapLimit, "region RAM overflowed")

    /* cortex-m-rt link.x.in places it in a different location I think */
    /* ASSERT( __binary_info_header_end - __logical_binary_start <= 256, "Binary info must be in first 256 bytes of the binary") */
    /* todo assert on extra code */
} INSERT BEFORE .text;
