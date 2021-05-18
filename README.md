# ExploreOS
This is a work-in-progress x86 operating system I am working on to explore the inner workings of operating systems, written entirely from scratch in Rust.
Currently an initial bootloader is finished, and on the kernel side memory management, interrupts and PS/2 keyboard and mouse drivers are all in a working state.

## Layout
* The Build Script (Resides at `src/main.rs`)
* The Bootloader (Resides at `bootloader/`)
* The Kernel (Resides at `kernel/`)
* Shared Libraries (Reside at `shared/*`)

## The Build Script
The script at `src/main.rs` builds the bootloader, builds the kernel, and assembles the os image.  
**NOTE**: The project currently requires the nightly channel of Rust.
- Run `cargo run` to build everything with `--release` and assemble the image `build/explore_os.img`.
- Run `cargo run kernel_debug` to build everything with `--release` except for the kernel which will be built using the `dev` profile.
- Run `cargo run release` to clean up the build directory.

## The Bootloader
The first stage of the bootloader, stage 0, resides at `bootloader/src/stage0/`, and is the place execution begins after the BIOS (the boot sector). This stage is responisble for reading the next (larger) stage from disk, switching to protected mode and finally passing off execution to the next bootloader stage.
The second stage of the bootloader resides at `bootloader/src/`. This stage initializes the serial ports for logging, builds up a physical memory map using the E820 BIOS call, reads the kernel from disk, sets up paging and a stack for the kernel, and finally jumps to the kernel.

## The Kernel
Execution begins at `kernel/src/main.rs` which first initializes a memory manager which is responisble for kernel allocations (both virtual and physical).Then a new GDT is initiailized to replace the one that was set up by stage 0 of the bootloader. A minimal TSS is also set up which is needed for stack switching when handling interrupts while in ring 3. Then the IDT is set up, the 8259A PIC is set up, and interrupts are enabled. The PS/2 controller is then initialized which in turn initializes the PS/2 keyboard and PS/2 mouse drivers if those devices are connected.

### Memory Manager
- Virtual memory is currently allocated using a simple bump allocator, with a free pages linked list. The map of the kernel's virtual address space is documented at `kernel/virt_mem_map.txt`.
- Unlike in the bootloader, where paging is disabled and physical memory can be accessed directly, access to physical pages in the kernel for editing page directories goes through an indirect route: The last page table, which is responisble for the mapping of the last page (at 0xFFFFF000) is permanently mapped in at 0xFFFFE000. When the kernel needs to edit the page mappings, the relevant page table is mapped in at the last page using the perm-mapped page table, and the relevant edits are made.

## Shared Libraries
- `compiler_reqs` - Basic memory functions required for compiling bare metal Rust
- `cpu` - x86-specific (assembly) routines
- `lock_cell` - Fair spin-lock for interior mutability
- `exclusive_cell` - Interior mutability where simultaneous access is not allowed  (panics if access is not exclusive)
- `serial` - Basic UART serial driver used for logging in both the bootloader and the kernel
- `range_set` - A set of non-overlapping and non-contiguous u32 inclusive ranges. Used to represent and allocate physical memory
- `elf_parser` - Minimal parser for ELF files used by the build script and by the bootloader to load the kernel
- `page_tables` - Functions for management of x86 32-bit paging
- `boot_args` - Holds common structure definition for the bootloader and kernel for passing during the initial boot process

## Testing
- `test_qemu.sh` and `test_bochs.sh` load the OS image as a hard drive on the respective emulators.
- `build_and_debug.sh` builds the OS with the dev profile for the kernel, and also starts the OS in qemu with a gdb server enabled.
- `debug_bootloader.sh` and `debug_kernel.sh` starts GDB with the relevant file and connects to the server started by qemu.

## Acknowledgements
The structure of the bootloader was initially based on gamozolabs' "Chocolate Milk OS".