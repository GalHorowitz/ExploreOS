align 8
; Real Mode GDT
RMGDT_START:
	dd 0, 0	; Null descriptor
RMGDT_CODE_SEG:	; Code segment descriptor
; Base Address is 0x0, Limit is 0xfffff
; Type/Access flags: present=1, priviliege=00, type=1, code=1, conforming=0, readable=1, accessed=0
; Other flags: granularity=0, 32-bit default=0, 64-bit seg=0, AVL=0
	dw 0xffff		; Limit (Bits 0-15)
	dw 0x0			; Base (Bits 0-15)
	db 0x0			; Base (Bits 16-23)
	db 10011010b	; Type and Access Flags
	db 00001111b	; Other Flags (4 bits), Limit (Bits 16-19)
	db 0x0			; Base (Bits 24-31)
RMGDT_DATA_SEG:	; Data segment descriptor
; Base Address is 0x0, Limit is 0xfffff
; Type/Access flags: present=1, priviliege=00, type=1, code=0, direction=0, writable=1, accessed=0
; Other flags: granularity=0, 32-bit default=0, 64-bit seg=0, AVL=0
	dw 0xffff		; Limit (Bits 0-15)
	dw 0x0			; Base (Bits 0-15)
	db 0x0			; Base (Bits 16-23)
	db 10010010b	; Type and Access Flags
	db 00001111b	; Other Flags (4 bits), Limit (Bits 16-19)
	db 0x0			; Base (Bits 24-31)
RMGDT_END:

; GDT Descriptor
RMGDT_DESCRIPTOR:
	dw RMGDT_END - RMGDT_START - 1	; sizeof(GDT)-1
	dd RMGDT_START					; Address of GDT

; Constants for segment registers
REAL_CODE_SEG equ RMGDT_CODE_SEG - RMGDT_START
REAL_DATA_SEG equ RMGDT_DATA_SEG - RMGDT_START