# ExploreOS
This is a work-in-progress x86 operating system I am working on to explore the inner workings of operating systems, written entirely from scratch in Rust.
Currently an initial bootloader is finished, and on the kernel side memory management, interrupts and a PS/2 keyboard driver are all in a working state.  
Documentation of how the project is laid out and build instructions are available at [`documentation.md`](documentation.md).

### Acknowledgements
The structure of the bootloader was initially based on gamozolabs' "Chocolate Milk OS".