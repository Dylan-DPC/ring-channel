use crate::buffer::*;
use alloc::boxed::Box;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use core::{ops::Deref, ptr::NonNull};
use crossbeam_utils::CachePadded;
use derivative::Derivative;

#[cfg(feature = "futures_api")]
use crate::waitlist::*;

#[cfg(feature = "futures_api")]
use core::task::Waker;

#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
pub(super) struct ControlBlock<T> {
    pub(super) senders: CachePadded<AtomicUsize>,
    pub(super) receivers: CachePadded<AtomicUsize>,
    pub(super) connected: AtomicBool,
    pub(super) buffer: RingBuffer<T>,

    #[cfg(feature = "futures_api")]
    pub(super) waitlist: Waitlist<Waker>,
}

impl<T> ControlBlock<T> {
    fn new(capacity: usize) -> Self {
        Self {
            senders: CachePadded::new(AtomicUsize::new(1)),
            receivers: CachePadded::new(AtomicUsize::new(1)),
            connected: AtomicBool::new(true),
            buffer: RingBuffer::new(capacity),

            #[cfg(feature = "futures_api")]
            waitlist: Waitlist::new(),
        }
    }
}

#[derive(Derivative)]
#[derivative(
    Debug(bound = ""),
    Clone(bound = ""),
    Eq(bound = ""),
    PartialEq(bound = "")
)]
pub(super) struct ControlBlockRef<T>(NonNull<ControlBlock<T>>);

impl<T> Unpin for ControlBlockRef<T> {}

impl<T> ControlBlockRef<T> {
    pub(super) fn new(capacity: usize) -> Self {
        ControlBlockRef(unsafe {
            NonNull::new_unchecked(Box::into_raw(Box::new(ControlBlock::new(capacity))))
        })
    }
}

impl<T> Deref for ControlBlockRef<T> {
    type Target = ControlBlock<T>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}

impl<T> Drop for ControlBlockRef<T> {
    fn drop(&mut self) {
        debug_assert!(!self.connected.load(Ordering::SeqCst));
        debug_assert_eq!(self.senders.load(Ordering::SeqCst), 0);
        debug_assert_eq!(self.receivers.load(Ordering::SeqCst), 0);

        unsafe { Box::from_raw(&**self as *const ControlBlock<T> as *mut ControlBlock<T>) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Void;
    use test_strategy::proptest;

    #[proptest]
    fn control_block_starts_connected() {
        let ctrl = ControlBlock::<Void>::new(1);
        assert!(ctrl.connected.load(Ordering::SeqCst));
    }

    #[proptest]
    fn control_block_starts_with_reference_counters_equal_to_one() {
        let ctrl = ControlBlock::<Void>::new(1);
        assert_eq!(ctrl.senders.load(Ordering::SeqCst), 1);
        assert_eq!(ctrl.receivers.load(Ordering::SeqCst), 1);
    }

    #[proptest]
    fn control_block_allocates_buffer_given_capacity(#[strategy(1..=10usize)] cap: usize) {
        let ctrl = ControlBlock::<Void>::new(cap);
        assert_eq!(ctrl.buffer.capacity(), cap);
    }
}
