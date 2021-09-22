#!/bin/sh
cd $(dirname $0)

sudo mkdir /mnt/test_ext2 || exit $?
sudo mount -o loop test_ext2.fs /mnt/test_ext2 || exit $?