load_next_stage:
	; Check if disk extensions are available
	mov ah, 0x41
	mov dl, [BOOT_DRIVE]
	mov bx, 0x55AA
	int 0x13
	cmp bx, 0xAA55
	jne no_disk_extenstions

	; Get drive parameters to figure out how many sectors we need to read
	mov ah, 0x48
	mov dl, [BOOT_DRIVE]
	lea si, [DRIVE_PARAMETERS_RESULT]
	int 0x13
	jc failed_to_get_drive_params

	; Ensure the BIOS reports sector number of 512-bytes sectors, else bail
	mov ax, [BYTES_PER_SECTOR]
	cmp ax, 512
	jne invalid_sector_size

	; Make sure we don't need to load more than 0xFFFF sectors
	mov ax, [NUM_SECTORS_HIGH]
	test ax, ax
	jnz too_many_sectors

	; Update the number of sectors to read in the DISK ADDRESS PACKET
	mov ax, [NUM_SECTORS_LOW]
	dec ax ; We don't read the boot sector
	mov [NUM_SECTORS_TO_READ], ax

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

failed_to_get_drive_params:
	lea si, [FAILED_TO_GET_DRIVE_PARAMS_STR]
	call print
	jmp $

invalid_sector_size:
	lea si, [INVALID_SECTOR_SIZE_STR]
	call print
	jmp $

too_many_sectors:
	lea si, [TOO_MANY_SECTORS_STR]
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

DRIVE_PARAMETERS_RESULT:
	dw 0x1A ; size of result buffer, 0x1A means to use v1.x version
	dw 0
	dd 0, 0, 0
	NUM_SECTORS_LOW: dd 0
	NUM_SECTORS_HIGH: dd 0
	BYTES_PER_SECTOR: dw 0

DISK_ADDRESS_PACKET:
	db 0x10 ; sizeof(DAP)
	db 0x0	; reserved
	NUM_SECTORS_TO_READ: dw 0x0
	dw 0x7e00	; This is the address that the sectors are written to. It is encoded in
	dw 0x0		; segment:offset, but the offset comes first, so the final address is 0x7e00
	dq 0x1 ; LBA of start sector

NO_DISK_EXTENSIONS_STR: db "FATAL: BIOS doesn't have disk extensions!", 0 
FAILED_TO_GET_DRIVE_PARAMS_STR: db "FATAL: Failed to get drive parameters!", 0 
INVALID_SECTOR_SIZE_STR: db "FATAL: Sector size is not 512!", 0
TOO_MANY_SECTORS_STR: db "FATAL: Too many sectors to read!", 0
FAILED_TO_READ_STR: db "FATAL: Failed to read disk!", 0
LANDED_STR: db "SUCCESS: Loaded the next bootloader stage.", 0