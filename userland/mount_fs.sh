#!/bin/sh
cd $(dirname $0)

sudo mkdir mnt_ext2 || exit $?
sudo mount -o loop test_ext2.fs ./mnt_ext2 || exit $?