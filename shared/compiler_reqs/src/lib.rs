//! Some basic memory functions required for compiling for bare metal Rust

#![no_std]

// TODO: All these routines can be optimized by doing the bulk of the work in dwords

/// libc `memset` implementation in Rust
/// 
/// ### Parameters
/// * `s` - Pointer to memory to set
/// * `c` - Bytes to fill memory with
/// * `n` - Number of bytes to set
///
/// ### Safety
/// `s` must be valid for writes of `n` bytes
#[no_mangle]
pub unsafe extern fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *s.add(i) = c as u8;
        i += 1;
    }
    s
}

/// libc `memcpy` implementation in Rust
/// 
/// ### Parameters
/// * `dest` - Pointer to memory to copy to
/// * `src`  - Pointer to memory to copy from
/// * `n`    - Number of bytes to copy
///
/// ### Safety
/// The source and destination buffers must not overlap. `dest` must be valid for writes of `n`
/// bytes and `src` must be valid for reads of `n` bytes
#[no_mangle]
pub unsafe extern fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *dest.add(i) = *src.add(i);
        i += 1;
    }
    dest
}

/// libc `memmove` implementation in Rust
/// 
/// ### Parameters
/// * `dest` - Pointer to memory to copy to
/// * `src`  - Pointer to memory to copy from
/// * `n`    - Number of bytes to copy
///
/// ### Safety
/// `dest` must be valid for writes of `n` bytes and `src` must be valid for reads of `n` bytes
#[no_mangle]
pub unsafe extern fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    // If the buffers do not overlap, or the overlap is such that src comes first,
    // a normal memcpy will work
    if src >= (dest as *const u8) || src.add(n) < (dest as *const u8) {
        return memcpy(dest, src, n);
    }

    // A backward copy handles the case where there is overlap and dest comes first
    let mut i = n;
    while i > 0 {
        *dest.add(i) = *src.add(i);
        i -= 1;
    }

    dest
}

/// libc `memcmp` implementation in Rust
/// 
/// ### Parameters
/// * `s1` - Pointer to memory to compare with `s2`
/// * `s2` - Pointer to memory to compare with `s1`
/// * `n`  - Number of bytes to compare
///
/// ### Safety
/// `s1` and `s2` must be valid for reads of `n` bytes
#[no_mangle]
pub unsafe extern fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let b1 = *s1.add(i);
        let b2 = *s2.add(i);
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
/// * `s1` - Pointer to byte sequence to compare with `s2`
/// * `s2` - Pointer to byte sequence to compare with `s1`
/// * `n`  - Number of bytes to compare
///
/// ### Safety
/// `s1` and `s2` must be valid for reads of `n` bytes
#[no_mangle]
pub unsafe extern fn bcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    memcmp(s1, s2, n)
}