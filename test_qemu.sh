#!/bin/sh

qemu-system-i386 -serial stdio -drive format=raw,file=build/explore_os.img -m 1G

