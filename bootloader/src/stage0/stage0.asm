[org 0x7c00]
[bits 16]

entry:
	; Far-jump to reset cs to 0
	jmp 0:reset_cs

; Resets environment and then jumps to a routine which loads the next bootloader stage from disk
reset_cs:
	; Set segments to known state and setup stack
	mov bp, 0x7c00
	xor ax, ax
	mov ds, ax
	mov es, ax
	mov ss, ax
	mov sp, bp

	; Clear direction flag
	cld

	; On startup dl contains the boot disk id
	mov [BOOT_DRIVE], dl 

	; Load the next bootloader stage
	call load_next_stage

	; Switch to protected mode
	jmp switch_to_protected_mode

BOOT_DRIVE: db 0

%include "disk_reading.asm"

; Switches to protected mode
switch_to_protected_mode:

	cli	; Disable interrupts
	cld ; Clear direction flag

	; Enable the A20 line
	in al, 0x92
	or al, 2
	out 0x92, al

	; Load the flat-map GDT
	lgdt [ds:GDT_DESCRIPTOR]

	; Make the switch to protected mode by settings the first bit of cr0
	mov eax, cr0
	or eax, 0x1
	mov cr0, eax

	; Set cs to the new code segment
	jmp CODE_SEG:protected_mode_landing_point

%include "gdt.asm"

[bits 32]
; Setup after switching to protected mode, and jumps to next bootloader stage
protected_mode_landing_point:
	; Update all data segment registers to the new segment index
	mov ax, DATA_SEG
	mov ds, ax
	mov ss, ax
	mov es, ax
	mov fs, ax
	mov gs, ax
	
	; Setup stack
	mov ebp, 0x7c00
	mov esp, ebp
	

	; Push the size argument
	push dword BOOTLOADER_SIZE
	; Push the boot drive id argument
	xor eax, eax
	mov al, byte [BOOT_DRIVE]
	push eax
	; Jump to the next bootloader stage's entry point (Defined using -D when assembling)
	call BOOTLOADER_ENTRY_POINT

times 510-($-$$) db 0xCC	; Padding to set the last 2 bytes in the boot sector
dw 0xAA55					; Boot sector magic

; Include the next bootloader stage
INCBIN "../../../build/bootloader.flat"

BOOTLOADER_SIZE equ ($-$$)