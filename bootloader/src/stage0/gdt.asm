align 8
; GDT (Flat memory model)
GDT_START:
	dd 0, 0 ; Null descriptor
GDT_CODE_SEG: ; Code segment descriptor
; Base Address is 0x0, Limit is 0xfffff (Because granularity is 4K, segment size is 4GB)
; Type/Access flags: present=1, priviliege=00, type=1, code=1, conforming=0, readable=1, accessed=0
; Other flags: granularity=1, 32-bit default=1, 64-bit seg=0, AVL=0
	dw 0xffff		; Limit (Bits 0-15)
	dw 0x0			; Base (Bits 0-15)
	db 0x0			; Base (Bits 16-23)
	db 10011010b	; Type and Access Flags
	db 11001111b	; Other Flags (4 bits), Limit (Bits 16-19)
	db 0x0			; Base (Bits 24-31)
GDT_DATA_SEG: ; Data segment descriptor
; Base Address is 0x0, Limit is 0xfffff (Because granularity is 4K, segment size is 4GB)
; Type/Access flags: present=1, priviliege=00, type=1, code=0, direction=0, writable=1, accessed=0
; Other flags: granularity=1, 32-bit default=1, 64-bit seg=0, AVL=0
	dw 0xffff		; Limit (Bits 0-15)
	dw 0x0			; Base (Bits 0-15)
	db 0x0			; Base (Bits 16-23)
	db 10010010b	; Type and Access Flags
	db 11001111b	; Other Flags (4 bits), Limit (Bits 16-19)
	db 0x0			; Base (Bits 24-31)
GDT_END:

; GDT Descriptor
GDT_DESCRIPTOR:
	dw GDT_END - GDT_START - 1	; sizeof(GDT)-1
	dd GDT_START				; Address of GDT

; Constants for segment registers
CODE_SEG equ GDT_CODE_SEG - GDT_START
DATA_SEG equ GDT_DATA_SEG - GDT_START