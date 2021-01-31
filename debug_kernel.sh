#!/bin/sh

gdb-multiarch -ex "set disassembly-flavor intel" -ex "set arch i386" \
 	-ex "file build/kernel/i586-unknown-linux-gnu/release/kernel" \
	-ex "target remote localhost:1234" -ex "set disassemble-next-line on" \
	-ex "show disassemble-next-line" -ex "b entry" -ex "c"
