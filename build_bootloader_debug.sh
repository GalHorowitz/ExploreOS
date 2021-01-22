#!/bin/sh

cargo run
# tail -n +2 bootloader/src/stage0.asm > build/temp_stage0.asm

# cd bootloader
# cd src
# nasm -f elf -o ../../build/new_os_boot.elf ../../build/temp_stage0.asm
# cd ..
# cd ..

# rm build/temp_stage0.asm
# ld.lld -o build/new_os.elf -Ttext 0x7c00 -nostdlib build/new_os_boot.elf 
qemu-system-i386 -serial stdio -drive format=raw,file=build/new_os.img -s -S
