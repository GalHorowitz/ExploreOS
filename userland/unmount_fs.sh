#!/bin/sh
cd $(dirname $0)

sudo umount /mnt/test_ext2 || exit $?
sudo rmdir /mnt/test_ext2 || exit $?