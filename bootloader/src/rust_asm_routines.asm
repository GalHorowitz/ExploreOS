; This file contains routines and trampolines to switch between 16-bit and 32-bit (TODO: 64 bit)
; modes. We can't do this in inline assembly because we can't switch the bitness of the intsructions
[bits 32]

global invoke_realmode_interrupt
invoke_realmode_interrupt:
	; Save register state
	pushad
	
	; Load real mode GDT
	lgdt [ds:RMGDT_DESCRIPTOR]

	; Reset selectors
	mov ax, REAL_DATA_SEG
	mov ds, ax
	mov es, ax
	mov fs, ax
	mov gs, ax
	mov ss, ax

	; Far-jump to set CS to real mode GDT code segment
	jmp REAL_CODE_SEG:set_cs_target

[bits 16]
set_cs_target:
	; Disable protected mode
	mov eax, cr0
	and eax, ~1
	mov cr0, eax

	; Clear segment registers
	xor ax, ax
	mov ds, ax
	mov es, ax
	mov fs, ax
	mov gs, ax
	mov ss, ax

	; Far-jump to set CS
	jmp (BOOTLOADER_BASE_ADDR>>4):(actual_interrupt_invoker - BOOTLOADER_BASE_ADDR)

actual_interrupt_invoker:
	; Get function arguments. Because of the PUSHAD we need to skip the first 8 values on the stack,
	; and we also need to skip the return address
	mov ebx, DWORD [esp + (9*4)]	; arg #1: interrupt_num
	mov eax, DWORD [esp + (10*4)]	; arg #2: regs

	; Setup the stack for an interrupt call. We must do this manually because the INT instruction
	; only takes an immediate.
	pushf	; Push the lower 16 bits of EFLAGS
	push cs	; Push the code segment of the return addres
	push WORD (interrupt_return_point - BOOTLOADER_BASE_ADDR) ; Push the return address

	; IVT entries are 4 bytes large, so to get an offset into the IVT we shift-left by 2
	shl ebx, 2
	
	; Setup a fake stack frame for an iret call so we can far-jump to the interrupt handler
	pushf 				; eflags
	push WORD [bx+2]	; ivt segment (cs)
	push WORD [bx]		; ivt offset  (ip)

	; Load the specified register state
	mov ecx, DWORD [eax + (1*4)]
	mov edx, DWORD [eax + (2*4)]
	mov ebx, DWORD [eax + (3*4)]
	mov ebp, DWORD [eax + (4*4)]
	mov esi, DWORD [eax + (5*4)]
	mov edi, DWORD [eax + (6*4)]
	mov eax, DWORD [eax]

	; We 'return' into the interrupt handler
	iret

interrupt_return_point:
	; Save the interrupt result register state
	push eax
	push ecx
	push edx
	push ebx
	push ebp
	push esi
	push edi
	pushfd
	push ds
	push es
	push fs
	push gs
	push ss

	mov eax, DWORD [esp + (8*4) + (5*2) + (10*4)] ; The first argument: `regs`
	
	; Save the register state in the struct the argument points to. We do this through the stack
	; because we want to preserve all the registers (except esp)
	pop WORD [eax + (8*4) + (4*2)]
	pop WORD [eax + (8*4) + (3*2)]
	pop WORD [eax + (8*4) + (2*2)]
	pop WORD [eax + (8*4) + (1*2)]
	pop WORD [eax + (8*4)]
	pop DWORD [eax + (7*4)]
	pop DWORD [eax + (6*4)]
	pop DWORD [eax + (5*4)]
	pop DWORD [eax + (4*4)]
	pop DWORD [eax + (3*4)]
	pop DWORD [eax + (2*4)]
	pop DWORD [eax + (1*4)]
	pop DWORD [eax]


	; == Switch back to protected mode == 

	; Load ds for lgdt
	mov ax, BOOTLOADER_BASE_ADDR >> 4
	mov ds, ax

	; Load the protected mode GDT
	lgdt [ds:(GDT_DESCRIPTOR - BOOTLOADER_BASE_ADDR)]

	; Make the switch to protected mode by settings the first bit of cr0
	mov eax, cr0
	or eax, 0x1
	mov cr0, eax

	; Set cs to the new code segment
	jmp CODE_SEG:protected_mode_landing_point

[bits 32]
protected_mode_landing_point:
	; Update all data segment registers
	mov ax, DATA_SEG
	mov ds, ax
	mov ss, ax
	mov es, ax
	mov fs, ax
	mov gs, ax

	; Restore caller register state
	popad

	ret

%include "rmgdt.asm"
%include "stage0/gdt.asm"

[bits 32]
global jump_to_kernel
jump_to_kernel:
	; [esp + 0x04] - kernel entry
	; [esp + 0x08] - kernel stack
	; [esp + 0x0c] - kernel param
	; [esp + 0x10] - new cr3

	; Set the new cr3
	mov eax, [esp + 0x10] ; new cr3
	mov cr3, eax

	; Set cr0 to a known state
	mov eax, cr0
	or eax, (1<<5)  ; Numeric Error (Native x87 FPU errors)
	or eax, (1<<16) ; Write Protect
	or eax, (1<<31) ; Paging
	mov cr0, eax

	mov eax, [esp + 0x04] ; kernel entry
	mov ebx, [esp + 0x0c] ; kernel param
	
	; Setup new stack
	mov ebp, [esp + 0x08] ; stack
	mov esp, ebp

	; Call kernel code
	push ebx
	call eax
kernel_return:
	; If we for some reason ever return, we halt
	cli
	hlt

