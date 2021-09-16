//! A circular queue for a single producer, single consumer scenario

#![no_std]
#![feature(const_maybe_uninit_assume_init, maybe_uninit_uninit_array, maybe_uninit_extra)]

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicUsize, Ordering};

// Circular queue for single producer, single consumer
pub struct ProducerConsumer<T, const SIZE: usize> {
    queue: UnsafeCell<[MaybeUninit<T>; SIZE]>,
    produce_idx: AtomicUsize,
    consume_idx: AtomicUsize,
    uncosumed_count: AtomicUsize,
}

// We make sure access from multiple threads is safe
unsafe impl<T, const SIZE: usize> Sync for ProducerConsumer<T, SIZE> {}

impl<T, const SIZE: usize> ProducerConsumer<T, SIZE>  {
    // Construct an empty queue
    pub const fn new() -> ProducerConsumer<T, SIZE> {
        ProducerConsumer {
            queue: UnsafeCell::new(MaybeUninit::uninit_array()),
            produce_idx: AtomicUsize::new(0),
            consume_idx: AtomicUsize::new(0),
            uncosumed_count: AtomicUsize::new(0),
        }
    }

    pub fn produce(&self, val: T) -> Option<()> {
        if self.uncosumed_count.load(Ordering::Relaxed) == SIZE {
            return None
        }

        // The atomicity of this operations aren't really relevant because there should only be one
        // producer...
        let idx = self.produce_idx.load(Ordering::Relaxed);
        self.produce_idx.store((idx + 1) % SIZE, Ordering::Relaxed);

        unsafe { (*self.queue.get())[idx] = MaybeUninit::new(val); }

        self.uncosumed_count.fetch_add(1, Ordering::Relaxed);

        Some(())
    }

    pub fn produce_blocking(&self, val: T) {
        while self.uncosumed_count.load(Ordering::Relaxed) == SIZE {
            core::hint::spin_loop();
        }

        // Only one producer, so `uncosumed_count` can only decrease further
        self.produce(val).unwrap()
    }

    pub fn consume(&self) -> Option<T> {
        if self.uncosumed_count.load(Ordering::Relaxed) == 0 {
            return None
        }

        // The atomicity of this operations aren't really relevant because there should only be one
        // cosumer...
        let idx = self.consume_idx.load(Ordering::Relaxed);
        self.consume_idx.store((idx + 1) % SIZE, Ordering::Relaxed);

        let item = unsafe { (*self.queue.get())[idx].assume_init_read() };

        self.uncosumed_count.fetch_sub(1, Ordering::Relaxed);

        Some(item)
    }

    pub fn consume_blocking(&self) -> T {
        while self.uncosumed_count.load(Ordering::Relaxed) == 0 {
            core::hint::spin_loop();
        }

        // Only one consumer, so `uncosumed_count` can only increase further
        self.consume().unwrap()
    }
}