//! Interior mutability where simultaneous access is not allowed and checked in run-time

#![no_std]

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};
use core::ops::{Drop, Deref, DerefMut};

/// Interior mutability with exclusivity enforced at runtime
pub struct ExclusiveCell<T> {
    /// The actual cell, for interior mutability
    cell: UnsafeCell<T>,
    /// Whether or not the cell is currently in use
    in_use: AtomicBool,
}

// We make sure access from multiple threads is safe (by completely disallowing access from multiple
// thread simultaneously)
unsafe impl<T> Sync for ExclusiveCell<T> {}

impl<T> ExclusiveCell<T> {
    /// Construct an ExclusiveCell holding the initial value `val`
    pub const fn new(val: T) -> ExclusiveCell<T> {
        ExclusiveCell {
            cell: UnsafeCell::new(val),
            in_use: AtomicBool::new(false)
        }
    }

    /// Acquire exclusive rights to the cell. The function panics if the cell is already in-use.
    pub fn acquire(&self) -> ExclusiveCellGuard<T> {
        let already_in_use = self.in_use.swap(true, Ordering::SeqCst);
        if already_in_use {
            panic!("Attempt to acquire rights to an ExclusiveCell which is already in-use");
        }

        ExclusiveCellGuard {
            cell: self
        }
    }
}

pub struct ExclusiveCellGuard<'a, T> {
    cell: &'a ExclusiveCell<T>
}

impl<'a, T> Drop for ExclusiveCellGuard<'a, T> {
    fn drop(&mut self) {
        self.cell.in_use.store(false, Ordering::SeqCst);
    }
}

impl<'a, T> Deref for ExclusiveCellGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            &*self.cell.cell.get()
        }
    }
}

impl<'a, T> DerefMut for ExclusiveCellGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            &mut *self.cell.cell.get()
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::ExclusiveCell;

    #[test]
    fn it_works() {
        let ex_cell = ExclusiveCell::new(1);
        {
            assert!(*ex_cell.acquire() == 1);
        }
        {
            *ex_cell.acquire() = 4;
        }
        assert!(*ex_cell.acquire() == 4);
    }

    #[test]
    #[should_panic]
    fn inner_dropped() {
        struct TestDrop;
        impl Drop for TestDrop {
            fn drop(&mut self) {
                panic!("drop");
            }
        }

        let ex_cell = ExclusiveCell::new(TestDrop);
        let _val = ex_cell.acquire();
    }

    #[test]
    #[should_panic]
    fn non_exclusive() {
        let ex_cell = ExclusiveCell::new(0u8);
        let mut a = ex_cell.acquire();
        let mut b = ex_cell.acquire();
        *a = 1;
        *b = 2;
    }
}
