MEMORY {
	RAM : ORIGIN = 0x7e00, LENGTH = 0x97E00
}

ENTRY(entry);

SECTIONS {
	.text : {
		*(.text .text.*)
	} > RAM

	.rodata : {
		*(.rodata .rodata.*)
	} > RAM

	.data : {
		*(.data .data.*)
	} > RAM

	/DISCARD/ : {
		*(.eh_frame)
	}
}