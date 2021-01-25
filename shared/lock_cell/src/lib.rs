//! A fair spin-lock for interior mutability

#![no_std]

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::ops::{Drop, Deref, DerefMut};

/// Spin lock for interior mutability. This lock must not be used in an interrupt context, or it
/// could dead-lock. Instead use a lock which disables interrupts.
pub struct LockCell<T> {
    /// The actuall cell, for interior mutability
    cell: UnsafeCell<T>,
    /// The next to text to be acquired
    available_ticket: AtomicUsize,
    /// The ticket of the current owner of the lock
    owner_ticket: AtomicUsize
}

// We make sure access from multiple threads is safe
unsafe impl<T> Sync for LockCell<T> {}

impl<T> LockCell<T>  {
    /// Construct a LockCell
    pub const fn new(val: T) -> LockCell<T> {
        LockCell {
            cell: UnsafeCell::new(val),
            available_ticket: AtomicUsize::new(0),
            owner_ticket: AtomicUsize::new(0)
        }
    }

    /// Acquire exclusive rights to the cell. The function blocks until it has rights.
    pub fn lock(&self) -> LockCellGuard<T> {
        // Get our ticket in the queue
        let ticket = self.available_ticket.fetch_add(1, Ordering::SeqCst);

        // Wait until it is our turn, i.e. we are the owner
        while ticket != self.owner_ticket.load(Ordering::SeqCst) {
            core::hint::spin_loop();
        }

        LockCellGuard {
            lock: self
        }
    }
}

pub struct LockCellGuard<'a, T> {
    lock: &'a LockCell<T>
}

impl<'a, T> Drop for LockCellGuard<'a, T> {
    fn drop(&mut self) {
        // Advance the queue so the next ticket becomes the owner
        self.lock.owner_ticket.fetch_add(1, Ordering::SeqCst);
    }
}

impl<'a, T> Deref for LockCellGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            &*self.lock.cell.get()
        }
    }
}

impl<'a, T> DerefMut for LockCellGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            &mut *self.lock.cell.get()
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::LockCell;

    #[test]
    fn it_works() {
        let lock_cell = LockCell::new(1);
        {
            assert!(*lock_cell.lock() == 1);
        }
        {
            *lock_cell.lock() = 4;
        }
        assert!(*lock_cell.lock() == 4);
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

        let lock_cell = LockCell::new(TestDrop);
        let _val = lock_cell.lock();
    }
}
