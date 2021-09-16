//! A fair spin-lock for interior mutability

#![no_std]

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};
use core::ops::{Drop, Deref, DerefMut};

/// Spin lock for interior mutability
pub struct LockCell<T> {
    /// The actuall cell, for interior mutability
    cell: UnsafeCell<T>,
    /// The next to text to be acquired
    available_ticket: AtomicUsize,
    /// The ticket of the current owner of the lock
    owner_ticket: AtomicUsize,
    /// Whether or not the lock should mask interrupts when held
    interruptable: bool,
}

// We make sure access from multiple threads is safe
unsafe impl<T> Sync for LockCell<T> {}

impl<T> LockCell<T>  {
    /// Construct a LockCell holding the initial value `val`. Hardware interrupts will be masked
    /// while the lock is held.
    pub const fn new(val: T) -> LockCell<T> {
        LockCell {
            cell: UnsafeCell::new(val),
            available_ticket: AtomicUsize::new(0),
            owner_ticket: AtomicUsize::new(0),
            interruptable: true
        }
    }

    /// Construct a LockCell holding the initial value `val`. Hardware interrupts will NOT be masked
    /// while the lock is held.
    pub const fn new_non_interruptable(val: T) -> LockCell<T> {
        LockCell {
            cell: UnsafeCell::new(val),
            available_ticket: AtomicUsize::new(0),
            owner_ticket: AtomicUsize::new(0),
            interruptable: false
        }
    }

    /// Acquire exclusive rights to the cell. The function blocks until it has rights.
    pub fn lock(&self) -> LockCellGuard<T> {
        // We only need to unmask interrupts when the lock is released if this is an interruptable
        // lock and interrupts were already unmasked
        let unmask_interrupts = if self.interruptable && cpu::get_if() {
            // If interrupts are unmaksed, we mask them for the duration of the critical section
            unsafe { cpu::cli(); }
            true
        } else {
            false
        };

        // Get our ticket in the queue
        let ticket = self.available_ticket.fetch_add(1, Ordering::SeqCst);

        // Wait until it is our turn, i.e. we are the owner
        while ticket != self.owner_ticket.load(Ordering::SeqCst) {
            core::hint::spin_loop();
        }

        LockCellGuard {
            lock: self,
            unmask_interrupts
        }
    }
}

pub struct LockCellGuard<'a, T> {
    lock: &'a LockCell<T>,
    /// Whether or not the interrupts were enabled when the lock was taken, used to re-enable
    /// interrupts when the lock is released
    unmask_interrupts: bool,
}

impl<'a, T> Drop for LockCellGuard<'a, T> {
    fn drop(&mut self) {
        // Advance the queue so the next ticket becomes the owner
        self.lock.owner_ticket.fetch_add(1, Ordering::SeqCst);

        // Unmask interrupts if needed
        if self.unmask_interrupts {
            unsafe { cpu::sti(); }
        }
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
        let lock_cell = LockCell::new_non_interruptable(1);
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

        let lock_cell = LockCell::new_non_interruptable(TestDrop);
        let _val = lock_cell.lock();
    }
}
