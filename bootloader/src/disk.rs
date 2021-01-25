use serial::println;
use alloc::vec::Vec;
use crate::real_mode::{invoke_realmode_interrupt, RegisterState};

/// The packet used to request a disk read from the BIOS
#[derive(Debug)]
#[repr(C)]
struct DiskAddressPacket {
	struct_size:            u8,
	_unused:                u8,
	sector_read_count:      u16,
	memory_buffer_offset:   u16,
	memory_buffer_segment:  u16,
	start_sector_offset:    u64
}

/// The number of sectors to read in each call to the BIOS. The buffer is allocated on the stack,
/// so this can't be too large.
const SECTOR_BUFFER_SIZE: u32 = 8;

pub fn read_kernel(boot_disk_id: u8, bootloader_size: u32) -> Option<Vec<u8>> {
	// Get the sector count of the boot disk. We cast to u32, because we don't have enough memory
	// to load more sectors than that anyway
	let disk_sector_count = get_disk_sector_count(boot_disk_id)? as u32;

    // Dividing the size by 512 while rounding up gives us the bootloader sector count
    let bootloader_sector_count = (bootloader_size + 511) / 512;
    // We assume that the rest of the sectors on disk are kernel sectors
    let kernel_sector_count = disk_sector_count - bootloader_sector_count;
    
    // Local stack buffer which is under the 64K limit that the BIOS can read to
    let mut sector_buffer = [0u8; 512*SECTOR_BUFFER_SIZE as usize];

	let mut kernel_image: Vec<u8> = Vec::with_capacity((kernel_sector_count * 512) as usize);

	// Read each kernel sector
    for sector_off in (0..kernel_sector_count).step_by(SECTOR_BUFFER_SIZE as usize) {
        // We either read `SECTOR_BUFFER_SIZE` sectors, or if we are at the end of the image, the
        // remaining sectors
        let sectors_to_read = core::cmp::min(SECTOR_BUFFER_SIZE, kernel_sector_count - sector_off);
        
        let mut disk_address_packet = DiskAddressPacket {
            struct_size: 0x10,
            _unused: 0,
            sector_read_count: sectors_to_read as u16,
            memory_buffer_offset: &mut sector_buffer as *mut _ as u16,
            memory_buffer_segment: 0,
            start_sector_offset: (bootloader_sector_count + sector_off) as u64
        };
    
        let mut register_context = RegisterState {
            eax: 0x4200,
            edx: boot_disk_id as u32,
            esi: &mut disk_address_packet as *mut DiskAddressPacket as u32,
            ..Default::default()
        };

        // Perform the extended BIOS read
        unsafe { invoke_realmode_interrupt(0x13, &mut register_context); }
    
        // CF is set on error
		if (register_context.eflags & 1) != 0 {
            println!("Failed to read drive sector (int 13h/ah=42h)");
            return None;
        }

        // Append the read sectors to the kernel image
		kernel_image.extend(&sector_buffer[..sectors_to_read as usize * 512]);
	}
    
    println!("Read kernel image: {} bytes, at {:#x?}", kernel_image.len(), kernel_image.as_ptr());
    Some(kernel_image)
}

/// The result of a int 13h/ah=48h BIOS call
#[derive(Default)]
#[repr(C)]
struct DriveParametersResult {
	struct_size:            u16,
	info_flags:             u16,
	phys_cylinder_count:    u32,
	phys_head_count:        u32,
	phys_sectors_per_track: u32,
	total_sector_count:     u64,
	bytes_per_sector:       u16
}

/// Gets the total sector count of the disk with id `disk_id`. Uses int 13h/ah=48h of the BIOS
fn get_disk_sector_count(disk_id: u8) -> Option<u64> {
	let mut drive_params = DriveParametersResult {
        struct_size: 0x1A, // A size of 0x1A means we use the v1.x version of this call
        ..Default::default()
    };

    let mut register_context = RegisterState {
        eax: 0x4800,
        edx: disk_id as u32,
        esi: &mut drive_params as *mut DriveParametersResult as u32,
        ..Default::default()
    };

    // Invoke the interrupt to get the drive info, we are only interested in the sector count
    unsafe { invoke_realmode_interrupt(0x13, &mut register_context); }

    // CF is set on error
    if (register_context.eflags & 1) != 0 {
        println!("Failed to get drive parameters (int 13h/ah=48h)");
        return None;
    }

    if drive_params.bytes_per_sector != 512 {
        println!("Boot disk uses non standard sector size");
        return None;
    }
	
	Some(drive_params.total_sector_count)
} 