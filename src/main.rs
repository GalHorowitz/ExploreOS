use std::error::Error;
use std::path::Path;
use std::process::Command;

use elf_parser::ElfParser;

// Creates a flattened image of the elf file at `file_path`. On success returns a tuple containing
// (entry point vaddr, image base, image bytes)
fn flatten_elf<P: AsRef<Path>>(file_path: P) -> Option<(usize, usize, Vec<u8>)> {
    let elf = std::fs::read(file_path).ok()?;
    
    let parser = ElfParser::parse(&elf)?;

    let mut program_start = None;
    let mut program_end = None; // Inclusive
    parser.for_segment(|vaddr, size, _init_bytes, _flags| {
        // Calculate the end of the segment. We sub before we add to prevent an overflow for a
        // segment that includes the last address.
        let segment_end = vaddr.checked_add(size.checked_sub(1)?)?;

        // Setup initial values
        if program_start.is_none() {
            program_start = Some(vaddr);
            program_end = Some(segment_end);
        } else {
            // Extend the start and end of the program to fit the segment
            program_start = Some(std::cmp::min(program_start.unwrap(), vaddr));
            program_end = Some(std::cmp::max(program_end.unwrap(), segment_end));
        }

        Some(())
    })?;

    // Ensure we determined the program boundries
    let program_start = program_start?;
    let program_end = program_end?;

    // Calculate full program size
    let program_size = (program_end-program_start).checked_add(1)?;

    // Zeroed flattened image
    let mut flattened = vec![0u8; program_size];

    parser.for_segment(|vaddr, size, init_bytes, _flags| {
        // Copy the segment into the flattened image
        let flat_offset = vaddr - program_start;
        flattened[flat_offset..flat_offset.checked_add(size)?].copy_from_slice(init_bytes);
        Some(())
    })?;

    // Make sure the entry point is valid (i.e. inside image bounds)
    if parser.entry_point < program_start || parser.entry_point > program_end {
        return None;
    }

    Some((parser.entry_point, program_start, flattened))
}

/// Ensure the command is installed and working. Runs `command` with `args` and ensure stdout
/// contains all `expected` strings.
fn ensure_installed(command: &str, args: &[&str], expected: &[&str]) -> Option<()> {
    // Run the command
    let result = Command::new(command).args(args).output().ok()?;

    // Make sure the command exited without error
    if !result.status.success() {
        return None;
    }

    // Convert stdout to string
    let stdout = std::str::from_utf8(&result.stdout).ok()?;

    // Make sure stdout contains all expected strings
    if expected.iter().all(|x| stdout.contains(x)) {
        Some(())
    }else{
        None
    }
}

fn main() -> Result<(), Box<dyn Error>>{
    // Ensure NASM assembler is installed
    ensure_installed("nasm", &["-v"], &["NASM version"]).ok_or("NASM missing")?;

    // Ensure rust and required targets are installed
    ensure_installed("rustup", &["target", "list"],
        &[
            "i586-unknown-linux-gnu (installed)",
            "x86_64-unknown-linux-gnu (installed)"
        ]).ok_or("rustup missing or targets i586-unknown-linux-gnu or x86_64-unknown-linux-gnu \
                  missing")?;
    
    // Ensure lld linker is installed
    ensure_installed("ld.lld", &["--version"], &["LLD"]);

    // Create build directories if they do not exist
    std::fs::create_dir_all("build")?;
    std::fs::create_dir_all("build/bootloader")?;

    let bootloader_build_dir = Path::new("build").join("bootloader").canonicalize()?;

    // Build the bootloader
    if !Command::new("cargo").current_dir("bootloader")
        .args(&["build", "--release", "--target-dir", bootloader_build_dir.to_str().unwrap()])
        .status()?.success() {
        return Err("Failed to build bootloader".into());
    }

    // Flatten the ELF image
    let bootloader_elf = bootloader_build_dir.join("i586-unknown-linux-gnu").join("release")
        .join("bootloader");
    let (entry_point, image_base, image_bytes) =
        flatten_elf(bootloader_elf).ok_or("Failed to flatten bootloader ELF")?;

    // Ensure the base address is right after the boot sector (0x7c00+512)
    if image_base != 0x7e00 {
        eprintln!("Bootloader base address: {:#x}", image_base);
        return Err("Unexpected bootloader base address".into());
    }

    // Ensure the bootloader is small enough (we don't want to overwrite BIOS data which start at
    // address 0x9fc00)
    let max_image_size = 0x9fc00 - 0x7e00;
    if image_bytes.len() > max_image_size {
        eprintln!("Bootloader size: {}/{}", image_bytes.len(), max_image_size);
        return Err("Bootloader too big".into());
    }

    // Write out the flattened bootloader image
    std::fs::write(Path::new("build").join("bootloader.flat"), image_bytes)?;

    // Assemble stage0
    let stage0 = Path::new("bootloader").join("src").join("stage0.asm");
    let bootfile = Path::new("build").join("new_os.boot");
    if !Command::new("nasm").args(&[
            "-f", "bin",
            "-o", bootfile.to_str().unwrap(),
            &format!("-DBOOTLOADER_ENTRY_POINT={}", entry_point),
            stage0.to_str().unwrap()
        ]).status()?.success() {
        return Err("Failed to assemble stage0".into());
    }

    Ok(())
}
