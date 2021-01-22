[bits 16]

load_next_stage:
	; Check if disk extensions are available
	mov ah, 0x41
	mov dl, [BOOT_DRIVE]
	mov bx, 0x55AA
	int 0x13
	cmp bx, 0xAA55
	jne no_disk_extenstions

	; Update the number of sectors to read in the DISK ADDRESS PACKET
	mov word [NUM_SECTORS_TO_READ], BOOTLOADER_SECTORS

	; Read the sectors to the address right after the boot sector
	mov ah, 0x42
	mov dl, [BOOT_DRIVE]
	lea si, [DISK_ADDRESS_PACKET]
	int 0x13
	jc failed_to_read

	; Print stage loading success message
	lea si, [LANDED_STR]
	call print

	ret

no_disk_extenstions:
	lea si, [NO_DISK_EXTENSIONS_STR]
	call print
	jmp $

failed_to_read:
	lea si, [FAILED_TO_READ_STR]
	call print
	jmp $

; Prints the null-terminated string in `si`
print:
	mov ah, 0xE
	mov bx, 0x0
print_loop:
	lodsb
	test al, al
	jz print_end
	int 0x10
	jmp print_loop
print_end:
	ret

DISK_ADDRESS_PACKET:
	db 0x10 ; sizeof(DAP)
	db 0x0	; reserved
	NUM_SECTORS_TO_READ: dw 0x0
	dw BOOTLOADER_BASE_ADDR	; This is the address that the sectors are written to. It is encoded in
	dw 0x0					; segment:offset, but the offset comes first.
	dq 0x1 ; LBA of start sector

NO_DISK_EXTENSIONS_STR: db "FATAL: BIOS doesn't have disk extensions!", 0 
FAILED_TO_GET_DRIVE_PARAMS_STR: db "FATAL: Failed to get drive parameters!", 0 
INVALID_SECTOR_SIZE_STR: db "FATAL: Sector size is not 512!", 0
TOO_MANY_SECTORS_STR: db "FATAL: Too many sectors to read!", 0
FAILED_TO_READ_STR: db "FATAL: Failed to read disk!", 0
LANDED_STR: db "SUCCESS: Loaded the next stage.", 0

; Sector count derived from the BOOTLOADE_SIZE in bytes. We divide by 512 while rounding up to get
; the sector count, and then subtract one to compensate for the boot sector which is already loaded
BOOTLOADER_SECTORS equ (((BOOTLOADER_SIZE + 511)/512) - 1)