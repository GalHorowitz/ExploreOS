//! Some basic memory functions required for compiling for bare metal Rust

#![no_std]

/// libc `memset` implementation in Rust
/// 
/// ### Parameters
/// 
/// * `s` - Pointer to memory to set
/// * `c` - Bytes to fill memory with
/// * `n` - Number of bytes to set
#[no_mangle]
pub unsafe extern fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *s.offset(i as isize) = c as u8;
        i += 1;
    }
    s
}

/// libc `memcpy` implementation in Rust
/// 
/// ### Parameters
/// 
/// * `dest` - Pointer to memory to copy to
/// * `src`  - Pointer to memory to copy from
/// * `n`    - Number of bytes to copy
#[no_mangle]
pub unsafe extern fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *dest.offset(i as isize) = *src.offset(i as isize);
        i += 1;
    }
    dest
}

/// libc `memmove` implementation in Rust
/// 
/// ### Parameters
/// 
/// * `dest` - Pointer to memory to copy to
/// * `src`  - Pointer to memory to copy from
/// * `n`    - Number of bytes to copy
#[no_mangle]
pub unsafe extern fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    // If the buffers do not overlap, or the overlap is such that src comes first,
    // a normal memcpy will work
    if src >= (dest as *const u8) || src.offset(n as isize) < (dest as *const u8) {
        return memcpy(dest, src, n);
    }

    // A backward copy handles the case where there is overlap and dest comes first
    let mut i = n;
    while i > 0 {
        *dest.offset(i as isize) = *src.offset(i as isize);
        i -= 1;
    }

    dest
}

/// libc `memcmp` implementation in Rust
/// 
/// ### Parameters
/// 
/// * `s1` - Pointer to memory to compare with `s2`
/// * `s2` - Pointer to memory to compare with `s1`
/// * `n`  - Number of bytes to compare
#[no_mangle]
pub unsafe extern fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let b1 = *s1.offset(i as isize);
        let b2 = *s2.offset(i as isize);
        if b1 != b2 {
            return (b2 as i32) - (b1 as i32);
        }
        i += 1;
    }
    0
}

/// `bcmp` implementation in Rust
/// 
/// ### Parameters
/// 
/// * `s1` - Pointer to byte sequence to compare with `s2`
/// * `s2` - Pointer to byte sequence to compare with `s1`
/// * `n`  - Number of bytes to compare
#[no_mangle]
pub unsafe extern fn bcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    return memcmp(s1, s2, n);
}