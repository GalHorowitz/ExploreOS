ENTRY(_start)

SECTIONS {
	.text 0x10000000 : {
		*(.text .text.*)
		*(.rodata .rodata.*)
	}

	.data BLOCK(0x1000) : {
		*(.data .data.*)
		*(.bss .bss.*)
		*(COMMON)
	}
}