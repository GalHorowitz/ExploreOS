#!/bin/sh

rm /tmp/bochs_drive.img
rm /tmp/bochs_drive.img.lock
dd if=/dev/zero of=/tmp/bochs_drive.img count=4096 bs=512
dd if=build/explore_os.img of=/tmp/bochs_drive.img conv=notrunc
bochs -q
