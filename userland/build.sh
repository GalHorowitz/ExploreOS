#!/bin/sh
cd $(dirname $0)

mkdir -p fs/bin

# ls
/usr/local/i386elfgcc/bin/i386-elf-gcc -fno-pie -ffreestanding -nostdlib -masm=intel -fno-asynchronous-unwind-tables -c ls.c -o ls.o || exit $?
/usr/local/i386elfgcc/bin/i386-elf-ld -o fs/bin/ls -Taligned_segments.ld -nostdlib ls.o -L/usr/local/i386elfgcc/lib/gcc/i386-elf/9.3.0/ -lgcc || exit $?
rm ls.o

# cat
/usr/local/i386elfgcc/bin/i386-elf-gcc -fno-pie -ffreestanding -nostdlib -masm=intel -fno-asynchronous-unwind-tables -c cat.c -o cat.o || exit $?
/usr/local/i386elfgcc/bin/i386-elf-ld -o fs/bin/cat -Taligned_segments.ld -nostdlib cat.o -L/usr/local/i386elfgcc/lib/gcc/i386-elf/9.3.0/ -lgcc || exit $?
rm cat.o

# shell
/usr/local/i386elfgcc/bin/i386-elf-gcc -fno-pie -ffreestanding -nostdlib -masm=intel -fno-asynchronous-unwind-tables -c shell.c -o shell.o || exit $?
/usr/local/i386elfgcc/bin/i386-elf-ld -o fs/bin/shell -Taligned_segments.ld -nostdlib shell.o -L/usr/local/i386elfgcc/lib/gcc/i386-elf/9.3.0/ -lgcc || exit $?
rm shell.o

if test ! -f "test_ext2.fs"; then
	echo "Creating new ext2 filesystem"
	dd if=/dev/zero of=test_ext2.fs bs=1024 count=1024 || exit $?
	mkfs.ext2 test_ext2.fs || exit $?
fi

./mount_fs.sh || exit $?
sudo rm -rf /mnt/test_ext2/*
sudo cp -r fs/* /mnt/test_ext2
./unmount_fs.sh || exit $?