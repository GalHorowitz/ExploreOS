//! Interrupts initialization and handling

mod pic_8259a;
mod pit_8254;

use exclusive_cell::ExclusiveCell;
use crate::gdt::KERNEL_CS_SELECTOR;
use serial::println;

const IDT_ENTRIES: usize = 256;

/// Struct to wrap IDT entries to so we can set the alignment to 8 bytes (best performance according
/// to the Intel manual)
#[derive(Clone, Copy)]
#[repr(C, align(8))]
struct IDTEntry(u64);

static IDT: ExclusiveCell<[IDTEntry; IDT_ENTRIES]> = ExclusiveCell::new([IDTEntry(0); IDT_ENTRIES]);

/// Initializes the IDT, the PIC and the PIT, and unmasks interrupts
pub fn init() {
    let mut idt = IDT.acquire();

    assert!((idt.as_ptr() as usize) & 7 == 0);

    // Setup the descriptors for exceptions
    idt[0] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_0_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[1] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_1_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[2] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_2_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[3] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_3_handler as u32, 3, true,
        DescriptorType::InterruptGate);
    idt[4] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_4_handler as u32, 3, true,
        DescriptorType::InterruptGate);
    idt[5] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_5_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[6] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_6_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[7] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_7_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    
    // TODO: Use a task gate for the double fault handler so we can handle kernel stack corruptino
    idt[8] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_8_handler as u32, 0, true,
        DescriptorType::InterruptGate);

    idt[9] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_9_handler as u32, 0, true,
        DescriptorType::InterruptGate);
    idt[10] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_10_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[11] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_11_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[12] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_12_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[13] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_13_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[14] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_14_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[16] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_16_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[17] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_17_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[18] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_18_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[19] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_19_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[20] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_20_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[21] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_21_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    
    // Setup the descriptor for the 8259A PICs
    idt[32] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_32_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[33] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_33_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[34] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_34_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[35] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_35_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[36] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_36_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[37] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_37_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[38] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_38_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[39] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_39_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[40] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_40_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[41] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_41_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[42] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_42_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[43] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_43_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[44] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_44_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[45] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_45_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[46] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_46_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    idt[47] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_47_handler as u32, 0,
        true, DescriptorType::InterruptGate);
    
    // Setup the descriptor for syscalls
    idt[0x67] = IDTEntry::new(KERNEL_CS_SELECTOR, interrupt_103_handler as u32, 3,
        true, DescriptorType::TrapGate);

    // Load the IDT
    unsafe {
        cpu::load_idt(idt.as_ptr() as u32, ((IDT_ENTRIES * 8) - 1) as u16);
    }

    // Enable the 8259A PIC
    pic_8259a::init();
    
    // Enable the 8254 PIT
    pit_8254::init();

    // Unmask hardware interrupts
    unsafe { cpu::sti(); }
}

#[allow(dead_code)]
pub enum DescriptorType { TaskGate, InterruptGate, TrapGate }

impl IDTEntry {
    /// Constructs the u64 representing an interrupt descriptor based on the given parameters
    /// 
    /// * `segment` - the segment selector to switch to when calling the handler
    /// * `entry_offset` - the offset into the segment of the handler
    /// * `priviliege` - the requested privilege level (0-3)
    /// * `protected_mode` - if true, the handler will stay in protected mode, else the cpu will switch
    ///                      to real mode before calling the handler
    /// * `typ` - the descriptor type
    const fn new(segment: u16, entry_offset: u32, privilege: u32, protected_mode: bool,
        typ: DescriptorType) -> Self {
        assert!(privilege < 4);
        
        let type_bits = match typ {
            DescriptorType::InterruptGate => 0,
            DescriptorType::TrapGate => 1,
            DescriptorType::TaskGate => unimplemented!()
        };
        
        let low_dword = ((segment as u32) << 16) | (entry_offset & 0xFFFF);
        let high_dword = (entry_offset & 0xFFFF0000) | (1 << 15) | (privilege << 13) |
            ((protected_mode as u32) << 11) | (3 << 9) | (type_bits << 8);
        
        IDTEntry(((high_dword as u64) << 32) | (low_dword as u64))
    }
}

/// General interrupt handler, each interrupt lands here after going through its specific gate
unsafe extern "cdecl" fn interrupt_handler(interrupt_number: u32, error_code: u32, eip: u32) {
    let interrupt_number = interrupt_number as u8;
    
    if interrupt_number >= pic_8259a::PIC_IRQ_OFFSET
        && interrupt_number < pic_8259a::PIC_IRQ_OFFSET + 8 {
        let irq = interrupt_number - pic_8259a::PIC_IRQ_OFFSET;
        if pic_8259a::handle_spurious_irq(irq) {
            println!("WARNING: Spurious PIC IRQ {}!", irq);
            return;
        }
        
        if irq == 0 {
            pit_8254::handle_interrupt();
        } else if irq == 1 {
            crate::ps2::keyboard::handle_interrupt();
        } else if irq == 12 {
            unimplemented!("Mouse interrupt");
        } else {
            println!("PIC IRQ {}", irq);
        }
        
        pic_8259a::send_eoi(irq);
        return;
    }
    
    // FIXME: This will dead-lock if the exception happened while the serial lock is held
    println!("Handling interrupt {} with code={} eip={:#010x}", interrupt_number, error_code, eip);

    match interrupt_number {
        0 => panic!("Divide Error Exception (#DE)"),
        1 => panic!("Debug Exception (#DB)"),
        2 => panic!("NMI Interrupt"),
        3 => panic!("Breakpoint Exception (#BP)"),
        4 => panic!("Overflow Exception (#OF)"),
        5 => panic!("BOUND Range Exceeded Exception (#BR)"),
        6 => panic!("Invalid Opcode Exception (#UD)"),
        7 => panic!("Device Not Available Exception (#NM)"),
        8 => panic!("Double Fault Exception (#DF)"),
        9 => panic!("Coprocessor Segment Overrun"),
        10 => panic!("Invalid TSS Exception (#TS)"),
        11 => panic!("Segment Not Present (#NP)"),
        12 => panic!("Stack Fault Exception (#SS)"),
        13 => panic!("General Protection Exception (#GP)"),
        14 => panic!("Page-Fault Exception (#PF) CR2={:#010x}", cpu::get_cr2()),
        16 => panic!("x87 FPU Floating-Point Error (#MF)"),
        17 => panic!("Alignment Check Exception (#AC)"),
        18 => panic!("Machine-Check Exception (#MC)"),
        19 => panic!("SIMD Floating-Point Exception (#XM)"),
        20 => panic!("Virtualization Exception (#VE)"),
        21 => panic!("Control Protection Exception (#CP)"),
        _ => panic!("Unrecognized Interrupt")
    }
}

#[derive(Debug)]
#[repr(C)]
struct PushADRegisterState {
    edi: u32,
    esi: u32,
    ebp: u32,
    esp: u32,
    ebx: u32,
    edx: u32,
    ecx: u32,
    eax: u32,
}

/// Syscall interrupt handler, int 0x67 lands here
unsafe extern "cdecl" fn syscall_interrupt_handler(register_state: &mut PushADRegisterState) {
    crate::println!("Syscall {:?}", register_state);
}

macro_rules! int_asm_no_err_code {
    ($x:literal) => {
        asm!("
                push eax                // Save `cdecl` caller-saved registers on the stack
                push ecx
                push edx
                mov eax, [esp + 12]     // Grab the return eip from the interrupt frame
                push eax                // Push arg 3: the interrupt's return eip
                push dword ptr 0        // Push arg 2: the fake error code
                push dword ptr {int_no} // Push arg 1: the interrupt number
                call {int_handler}      // Call the handler function
                add esp, 12             // Pop the interrupt number, the error code, and the ret eip
                pop edx                 // Restore caller-saved registers
                pop ecx
                pop eax
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
                push eax                // Save `cdecl` caller-saved registers on the stack
                push ecx
                push edx
                mov eax, [esp + 16]     // Grab the return eip from the interrupt frame
                mov ecx, [esp + 12]     // Grab the interrupt error code
                push eax                // Push arg 3: the interrupt's return eip
                push ecx                // Push arg 2: the error code
                push dword ptr {int_no} // Push arg 1: the interrupt number
                call {int_handler}      // Call the handler function
                add esp, 8              // Pop the interrupt number and the error code
                add esp, 12             // Pop the interrupt number, the error code, and the ret eip
                pop edx                 // Restore caller-saved registers
                pop ecx
                pop eax
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
unsafe extern fn interrupt_32_handler() -> ! {
    int_asm_no_err_code!(32);
}

#[naked]
unsafe extern fn interrupt_33_handler() -> ! {
    int_asm_no_err_code!(33);
}

#[naked]
unsafe extern fn interrupt_34_handler() -> ! {
    int_asm_no_err_code!(34);
}

#[naked]
unsafe extern fn interrupt_35_handler() -> ! {
    int_asm_no_err_code!(35);
}

#[naked]
unsafe extern fn interrupt_36_handler() -> ! {
    int_asm_no_err_code!(36);
}

#[naked]
unsafe extern fn interrupt_37_handler() -> ! {
    int_asm_no_err_code!(37);
}

#[naked]
unsafe extern fn interrupt_38_handler() -> ! {
    int_asm_no_err_code!(38);
}

#[naked]
unsafe extern fn interrupt_39_handler() -> ! {
    int_asm_no_err_code!(39);
}

#[naked]
unsafe extern fn interrupt_40_handler() -> ! {
    int_asm_no_err_code!(40);
}

#[naked]
unsafe extern fn interrupt_41_handler() -> ! {
    int_asm_no_err_code!(41);
}

#[naked]
unsafe extern fn interrupt_42_handler() -> ! {
    int_asm_no_err_code!(42);
}

#[naked]
unsafe extern fn interrupt_43_handler() -> ! {
    int_asm_no_err_code!(43);
}

#[naked]
unsafe extern fn interrupt_44_handler() -> ! {
    int_asm_no_err_code!(44);
}

#[naked]
unsafe extern fn interrupt_45_handler() -> ! {
    int_asm_no_err_code!(45);
}

#[naked]
unsafe extern fn interrupt_46_handler() -> ! {
    int_asm_no_err_code!(46);
}

#[naked]
unsafe extern fn interrupt_47_handler() -> ! {
    int_asm_no_err_code!(47);
}

#[naked]
unsafe extern fn interrupt_103_handler() -> ! {
    asm!("
            pushad
            push esp            // Function argument: the pushad register state
            call {int_handler}  // Call the handler function
            add esp, 4          // Pop the argument
            popad
            iretd               // Return from the interrupt
        ",
        int_handler = sym syscall_interrupt_handler,
        options(noreturn)
    );
}