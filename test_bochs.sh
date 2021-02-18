#!/bin/sh

dd if=/dev/zero of=/tmp/bochs_disk.img count=1008 bs=512
dd if=build/new_os.img of=/tmp/bochs_disk.img conv=notrunc
bochs -q
