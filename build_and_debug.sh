#!/bin/sh

cargo run

qemu-system-i386 -serial stdio -drive format=raw,file=build/new_os.img -s -S
