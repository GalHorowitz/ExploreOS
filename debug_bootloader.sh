#!/bin/sh

gdb-multiarch -ex "set disassembly-flavor intel" -ex "set arch i8086" -ex "file build/new_os.elf" \
	-ex "target remote localhost:1234" -ex "set disassemble-next-line on" \
	-ex "show disassemble-next-line" -ex "b *0x7c00" -ex "c"