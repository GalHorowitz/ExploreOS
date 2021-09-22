#!/bin/sh
cd $(dirname $0)

# ls
/usr/local/i386elfgcc/bin/i386-elf-gcc -fno-pie -ffreestanding -nostdlib -masm=intel -fno-asynchronous-unwind-tables -c ls.c -o ls.o || exit $?
/usr/local/i386elfgcc/bin/i386-elf-ld -o fs/bin/ls -Taligned_segments.ld -nostdlib ls.o -L/usr/local/i386elfgcc/lib/gcc/i386-elf/9.3.0/ -lgcc || exit $?
rm ls.o

# shell
/usr/local/i386elfgcc/bin/i386-elf-gcc -fno-pie -ffreestanding -nostdlib -masm=intel -fno-asynchronous-unwind-tables -c shell.c -o shell.o || exit $?
/usr/local/i386elfgcc/bin/i386-elf-ld -o fs/bin/shell -Taligned_segments.ld -nostdlib shell.o -L/usr/local/i386elfgcc/lib/gcc/i386-elf/9.3.0/ -lgcc || exit $?
rm shell.o

./mount_fs.sh || exit $?
sudo rm -rf /mnt/test_ext2/*
sudo cp -r fs/* /mnt/test_ext2
./unmount_fs.sh || exit $?