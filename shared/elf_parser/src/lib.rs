//! Minimal parser for ELF files. This is not a general-purpose ELF parser; it only parses the
//! information we care about.

#![no_std]

use core::convert::TryInto;

pub const ELF_TYPE_ET_EXEC: u16 = 2;
pub const ELF_MACHINE_X86: u16 = 3;
pub const SEGMENT_TYPE_PT_LOAD: u32 = 1;
pub const SEGMENT_FLAGS_PF_X: u32 = 1;
pub const SEGMENT_FLAGS_PF_W: u32 = 2;
pub const SEGMENT_FLAGS_PF_R: u32 = 4;
pub const ELF_PROGRAM_HEADER_32_SIZE: usize = 0x20;

/// A validated ELF file
pub struct ElfParser<'a> {
    /// Virtual address of the code entry point
    pub entry_point: usize,

    /// Number of segments
    segment_count: usize,

    /// Offset into the file where the segment headers reside
    segment_headers_offset: usize,

    /// Raw ELF file
    raw_bytes: &'a [u8],
}

impl<'a> ElfParser<'a> {
    /// Validate the passed ELF file, and return a processed version of the ELF which can be queried
    /// for information about the ELF.
    pub fn parse(bytes: &'a [u8]) -> Option<Self> {
        // Check for ELF magic
        if bytes.get(0..4) != Some(b"\x7fELF") {
            return None;
        }

        // Check the ELF header fits 
        if bytes.len() < 58 {
            return None;
        }

        // Check this is a 32-bit ELF
        if bytes[4] != 1 {
            return None;
        }

        // Check this is a little-endian ELF
        if bytes[5] != 1 {
            return None;
        }

        // Check that the elf type is `ET_EXEC`
        if u16::from_le_bytes(bytes[16..18].try_into().ok()?) != ELF_TYPE_ET_EXEC {
            return None;
        }

        // Check that the machine type is x86
        if u16::from_le_bytes(bytes[18..20].try_into().ok()?) != ELF_MACHINE_X86 {
            return None;
        }

        // Get the entry point address
        let entry_point: usize = u32::from_le_bytes(bytes[24..28].try_into().ok()?)
            .try_into().ok()?;

        // Get the file offset to program headers
        let program_header_offset: usize = u32::from_le_bytes(bytes[28..32].try_into().ok()?)
            .try_into().ok()?;

        // Get the number of program headers in the file
        let program_header_count: usize = u16::from_le_bytes(bytes[44..46].try_into().ok()?)
            .try_into().ok()?;

        // Compute the total headers size of program headers and make sure we got enough bytes
        let program_headers_end = program_header_offset.checked_add(
            program_header_count.checked_mul(ELF_PROGRAM_HEADER_32_SIZE)?)?;
        if program_headers_end > bytes.len() {
            return None;
        }

        Some(ElfParser {
            entry_point,
            segment_count: program_header_count,
            segment_headers_offset: program_header_offset,
            raw_bytes: bytes,
        })
    }

    /// Invokes the provided closure with the details of every LOAD segment in the ELF
    /// The arguments are (virtual address, virtual size, raw init bytes, segment flags)
    pub fn for_segment<F>(&self, mut func: F) -> Option<()>
        where F: FnMut(usize, usize, &[u8], u32) -> Option<()> {
        let bytes = self.raw_bytes;

        for segment_idx in 0..self.segment_count {
            let off = self.segment_headers_offset + ELF_PROGRAM_HEADER_32_SIZE*segment_idx;
            
            // We only care about loaded segments
            if u32::from_le_bytes(bytes[off..off+4].try_into().ok()?) != SEGMENT_TYPE_PT_LOAD {
                continue;
            }

            // Get the file offset of the segment bytes
            let seg_file_offset: usize =
                u32::from_le_bytes(bytes[off+4..off+8].try_into().ok()?).try_into().ok()?;
            
            // Get the virtual address of the segment in memory
            let seg_vaddr: usize =
                u32::from_le_bytes(bytes[off+8..off+12].try_into().ok()?).try_into().ok()?;
            
            // Get the size of the segment bytes in the file
            let seg_file_bytes_size: usize =
                u32::from_le_bytes(bytes[off+16..off+20].try_into().ok()?).try_into().ok()?;
            
            // Get the size of the segment in memory
            let seg_mem_size: usize =
                u32::from_le_bytes(bytes[off+20..off+24].try_into().ok()?).try_into().ok()?;

            // Get the segment flags (R/W/X)
            let seg_flags = u32::from_le_bytes(bytes[off+24..off+28].try_into().ok()?);

            func(seg_vaddr, seg_mem_size,
                &bytes[seg_file_offset..seg_file_offset+seg_file_bytes_size], seg_flags)?;
        }

        Some(())
    }
}

#[cfg(test)]
mod tests {

    use crate::*;
    extern crate std;

    #[test]
    fn works() {
        let file = std::fs::read("../../build/kernel/i586-unknown-linux-gnu/release/kernel").unwrap();
        let parser = ElfParser::parse(&file).unwrap();
        parser.for_segment(|vaddr, vsize, raw_bytes, flags| {
            std::println!("{:#09x} {} {:03b}", vaddr, vsize, flags);
            std::println!("{:x?}", raw_bytes);
            Some(())
        }).unwrap();
    }
}