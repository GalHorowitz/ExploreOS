#!/bin/sh

cargo run kernel_debug

qemu-system-i386 -serial stdio -drive format=raw,file=build/new_os.img -s -S
