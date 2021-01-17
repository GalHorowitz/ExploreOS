#!/bin/sh

cargo run
qemu-system-i386 -serial stdio build/new_os.boot
