use alloc::alloc::{alloc_zeroed, Layout};
use serial::println;

#[allow(dead_code)]
pub enum DescriptorType { TaskGate, InterruptGate, TrapGate }

/// Constructs the u64 representing an interrupt descriptor based on the given parameters
/// 
/// * `segment` - the segment selector to switch to when calling the handler
/// * `entry_offset` - the offset into the segment of the handler
/// * `priviliege` - the requested privilege level (0-3)
/// * `protected_mode` - if true, the handler will stay in protected mode, else the cpu will switch
///                      to real mode before calling the handler
/// * `typ` - the descriptor type
pub fn construct_interrupt_descriptor(segment: u16, entry_offset: u32, privilege: u32,
    protected_mode: bool, typ: DescriptorType) -> u64 {
    assert!(privilege < 4);
    
    let type_bits = match typ {
        DescriptorType::InterruptGate => 0,
        DescriptorType::TrapGate => 1,
        DescriptorType::TaskGate => unimplemented!()
    };
    
    let low_dword = ((segment as u32) << 16) | (entry_offset & 0xFFFF);
    let high_dword = (entry_offset & 0xFFFF0000) | (1 << 15) | (privilege << 13) |
        ((protected_mode as u32) << 11) | (3 << 9) | (type_bits << 8);
    
    ((high_dword as u64) << 32) | (low_dword as u64)
}

/// General interrupt handler, each interrupt lands here after going through its specific gate
unsafe extern "cdecl" fn interrupt_handler(interrupt_number: u32, error_code: u32) {
    println!("Handling interrupt {} with code={}", interrupt_number, error_code);
    match interrupt_number {
        0 => println!("Divide Error Exception (#DE)"),
        1 => println!("Debug Exception (#DB)"),
        2 => println!("NMI Interrupt"),
        3 => println!("Breakpoint Exception (#BP)"),
        4 => println!("Overflow Exception (#OF)"),
        5 => println!("BOUND Range Exceeded Exception (#BR)"),
        6 => println!("Invalid Opcode Exception (#UD)"),
        7 => println!("Device Not Available Exception (#NM)"),
        8 => println!("Double Fault Exception (#DF)"),
        9 => println!("Coprocessor Segment Overrun"),
        10 => println!("Invalid TSS Exception (#TS)"),
        11 => println!("Segment Not Present (#NP)"),
        12 => println!("Stack Fault Exception (#SS)"),
        13 => println!("General Protection Exception (#GP)"),
        14 => println!("Page-Fault Exception (#PF)"),
        16 => println!("x87 FPU Floating-Point Error (#MF)"),
        17 => println!("Alignment Check Exception (#AC)"),
        18 => println!("Machine-Check Exception (#MC)"),
        19 => println!("SIMD Floating-Point Exception (#XM)"),
        20 => println!("Virtualization Exception (#VE)"),
        21 => println!("Control Protection Exception (#CP)"),
        103 => println!("SYSCALL"),
        _ => panic!("Unrecognized Interrupt")
    }
    cpu::halt();
}

/// Initializes the IDT
pub fn init() {
    const IDT_ENTRIES: usize = 256;

    // Allocate the table which according to the intel manual should be 8-byte aligned for best
    // performance
    let idt = unsafe {
        let idt_ptr = alloc_zeroed(Layout::from_size_align(IDT_ENTRIES * 8, 8).unwrap());
        core::slice::from_raw_parts_mut(idt_ptr as *mut u64, IDT_ENTRIES)
    };

    // Setup the descriptors for exceptions
    idt[0] = construct_interrupt_descriptor(0x8, interrupt_0_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[1] = construct_interrupt_descriptor(0x8, interrupt_1_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[2] = construct_interrupt_descriptor(0x8, interrupt_2_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[3] = construct_interrupt_descriptor(0x8, interrupt_3_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[4] = construct_interrupt_descriptor(0x8, interrupt_4_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[5] = construct_interrupt_descriptor(0x8, interrupt_5_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[6] = construct_interrupt_descriptor(0x8, interrupt_6_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[7] = construct_interrupt_descriptor(0x8, interrupt_7_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[8] = construct_interrupt_descriptor(0x8, interrupt_8_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[9] = construct_interrupt_descriptor(0x8, interrupt_9_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[10] = construct_interrupt_descriptor(0x8, interrupt_10_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[11] = construct_interrupt_descriptor(0x8, interrupt_11_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[12] = construct_interrupt_descriptor(0x8, interrupt_12_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[13] = construct_interrupt_descriptor(0x8, interrupt_13_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[14] = construct_interrupt_descriptor(0x8, interrupt_14_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[16] = construct_interrupt_descriptor(0x8, interrupt_16_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[17] = construct_interrupt_descriptor(0x8, interrupt_17_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[18] = construct_interrupt_descriptor(0x8, interrupt_18_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[19] = construct_interrupt_descriptor(0x8, interrupt_19_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[20] = construct_interrupt_descriptor(0x8, interrupt_20_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[21] = construct_interrupt_descriptor(0x8, interrupt_21_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    
    // Setup the descriptor for system calls
    idt[0x67] = construct_interrupt_descriptor(0x8, interrupt_103_handler as u32, 0, true,
        DescriptorType::InterruptGate);

    // Load the IDT
    unsafe {
        cpu::load_idt(idt.as_ptr() as u32, (IDT_ENTRIES*8 - 1) as u16);
    }
}

macro_rules! int_asm_no_err_code {
    ($x:literal) => {
        asm!("
                push dword ptr 0        // Push fake error code
                push dword ptr {int_no} // Push the interrupt number
                call {int_handler}      // Call the handler function
                add esp, 8              // Pop the interrupt number and the error code
                iretd                   // Return from the interrupt
            ",
            int_no = const $x,
            int_handler = sym interrupt_handler,
            options(noreturn)
        );
    }
}

macro_rules! int_asm_err_code {
    ($x:literal) => {
        asm!("
                push dword ptr {int_no} // Push the interrupt number
                call {int_handler}      // Call the handler function
                add esp, 8              // Pop the interrupt number and the error code
                iretd                   // Return from the interrupt
            ",
            int_no = const $x,
            int_handler = sym interrupt_handler,
            options(noreturn)
        );
    }
}

#[naked]
unsafe extern fn interrupt_0_handler() -> ! {
    int_asm_no_err_code!(0);
}

#[naked]
unsafe extern fn interrupt_1_handler() -> ! {
    int_asm_no_err_code!(1);
}

#[naked]
unsafe extern fn interrupt_2_handler() -> ! {
    int_asm_no_err_code!(2);
}

#[naked]
unsafe extern fn interrupt_3_handler() -> ! {
    int_asm_no_err_code!(3);
}

#[naked]
unsafe extern fn interrupt_4_handler() -> ! {
    int_asm_no_err_code!(4);
}

#[naked]
unsafe extern fn interrupt_5_handler() -> ! {
    int_asm_no_err_code!(5);
}

#[naked]
unsafe extern fn interrupt_6_handler() -> ! {
    int_asm_no_err_code!(6);
}

#[naked]
unsafe extern fn interrupt_7_handler() -> ! {
    int_asm_no_err_code!(7);
}

#[naked]
unsafe extern fn interrupt_8_handler() -> ! {
    int_asm_err_code!(8);
}

#[naked]
unsafe extern fn interrupt_9_handler() -> ! {
    int_asm_no_err_code!(9);
}

#[naked]
unsafe extern fn interrupt_10_handler() -> ! {
    int_asm_err_code!(10);
}

#[naked]
unsafe extern fn interrupt_11_handler() -> ! {
    int_asm_err_code!(11);
}

#[naked]
unsafe extern fn interrupt_12_handler() -> ! {
    int_asm_err_code!(12);
}

#[naked]
unsafe extern fn interrupt_13_handler() -> ! {
    int_asm_err_code!(13);
}

#[naked]
unsafe extern fn interrupt_14_handler() -> ! {
    int_asm_err_code!(14);
}

#[naked]
unsafe extern fn interrupt_16_handler() -> ! {
    int_asm_no_err_code!(16);
}

#[naked]
unsafe extern fn interrupt_17_handler() -> ! {
    int_asm_err_code!(17);
}

#[naked]
unsafe extern fn interrupt_18_handler() -> ! {
    int_asm_no_err_code!(18);
}

#[naked]
unsafe extern fn interrupt_19_handler() -> ! {
    int_asm_no_err_code!(19);
}

#[naked]
unsafe extern fn interrupt_20_handler() -> ! {
    int_asm_no_err_code!(20);
}

#[naked]
unsafe extern fn interrupt_21_handler() -> ! {
    int_asm_err_code!(21);
}

#[naked]
unsafe extern fn interrupt_103_handler() -> ! {
    int_asm_no_err_code!(103);
}