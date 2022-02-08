#!/bin/sh
cd $(dirname $0)

sudo umount ./mnt_ext2 || exit $?
sudo rmdir mnt_ext2 || exit $?