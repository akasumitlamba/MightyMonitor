//! Original Rust context storage for zero-initialized WDF allocations.
//! Native users must hold a WDF reference (or a framework callback's live handle)
//! throughout every operation. Only EvtDestroy may call destroy().
#![deny(unsafe_op_in_unsafe_fn)]
use std::{
    ptr,
    sync::{
        atomic::{AtomicBool, AtomicPtr, Ordering},
        Arc, RwLock,
    },
};
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    Closing,
    Uninitialized,
    AlreadyInitialized,
}
#[repr(C)]
pub struct ContextSlot<T> {
    closing: AtomicBool,
    value: AtomicPtr<RwLock<T>>,
}
impl<T> Default for ContextSlot<T> {
    fn default() -> Self {
        Self {
            closing: AtomicBool::new(false),
            value: AtomicPtr::new(ptr::null_mut()),
        }
    }
}
impl<T> ContextSlot<T> {
    pub fn mark_closing(&self) {
        self.closing.store(true, Ordering::Release);
    }
    pub fn is_closing(&self) -> bool {
        self.closing.load(Ordering::Acquire)
    }
    /// # Safety
    /// Keep the native allocation referenced until this call returns. Destruction
    /// must not run concurrently. WDF zeroed storage is a valid empty slot.
    pub unsafe fn install(&self, value: Arc<RwLock<T>>) -> Result<(), Error> {
        if self.is_closing() {
            return Err(Error::Closing);
        }
        let raw = Arc::into_raw(value).cast_mut();
        if self
            .value
            .compare_exchange(ptr::null_mut(), raw, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            // SAFETY: this raw reference was not installed; consume it once.
            unsafe {
                drop(Arc::from_raw(raw));
            }
            return Err(Error::AlreadyInitialized);
        }
        if self.is_closing() {
            self.value.store(ptr::null_mut(), Ordering::Release);
            // SAFETY: caller prevents destruction; install is the sole writer.
            unsafe {
                drop(Arc::from_raw(raw));
            }
            return Err(Error::Closing);
        }
        Ok(())
    }
    /// # Safety
    /// Caller holds a native reference so EvtDestroy cannot free the raw Arc
    /// between loading its address and incrementing its strong count.
    pub unsafe fn borrow(&self) -> Result<Arc<RwLock<T>>, Error> {
        if self.is_closing() {
            return Err(Error::Closing);
        }
        let value = unsafe { self.borrow_during_cleanup()? };
        if self.is_closing() {
            return Err(Error::Closing);
        }
        Ok(value)
    }
    /// # Safety
    /// Framework cleanup callback only, with a live native object reference.
    /// Unlike ordinary borrows, permits closing a partially initialized context.
    pub unsafe fn borrow_during_cleanup(&self) -> Result<Arc<RwLock<T>>, Error> {
        let raw = self.value.load(Ordering::Acquire);
        if raw.is_null() {
            return Err(Error::Uninitialized);
        }
        // SAFETY: caller's native reference excludes concurrent EvtDestroy.
        unsafe {
            Arc::increment_strong_count(raw);
        }
        Ok(unsafe { Arc::from_raw(raw) })
    }
    /// # Safety
    /// EvtDestroy only: native references are zero, so no borrow/install can run.
    /// Handles partial creation and repeated cleanup without reading an invalid Arc.
    pub unsafe fn destroy(&self) {
        self.mark_closing();
        let raw = self.value.swap(ptr::null_mut(), Ordering::AcqRel);
        if !raw.is_null() {
            // SAFETY: consumes exactly the strong reference installed in this slot.
            unsafe {
                drop(Arc::from_raw(raw));
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Count(Arc<std::sync::atomic::AtomicUsize>);
    impl Drop for Count {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    #[test]
    fn cleanup_blocks_new_borrows_without_destroying_inflight_data() {
        let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let slot = ContextSlot::default();
        // SAFETY: a live test allocation simulates one pinned native context;
        // destruction happens only after the borrow operation itself has finished.
        unsafe {
            slot.install(Arc::new(RwLock::new(Count(count.clone()))))
                .unwrap();
        }
        let borrowed = unsafe { slot.borrow().unwrap() };
        slot.mark_closing();
        assert!(matches!(unsafe { slot.borrow() }, Err(Error::Closing)));
        unsafe {
            slot.destroy();
            slot.destroy();
        }
        assert_eq!(count.load(Ordering::SeqCst), 0);
        drop(borrowed);
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn partial_creation_and_duplicate_initialization_are_safe() {
        let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let slot = ContextSlot::<Count>::default();
        assert!(matches!(
            unsafe { slot.borrow() },
            Err(Error::Uninitialized)
        ));
        unsafe {
            slot.install(Arc::new(RwLock::new(Count(count.clone()))))
                .unwrap();
        }
        assert_eq!(
            unsafe { slot.install(Arc::new(RwLock::new(Count(count.clone())))) },
            Err(Error::AlreadyInitialized)
        );
        assert_eq!(count.load(Ordering::SeqCst), 1);
        unsafe {
            slot.destroy();
        }
        assert_eq!(count.load(Ordering::SeqCst), 2);
        let empty = ContextSlot::<Count>::default();
        unsafe {
            empty.destroy();
        }
        assert_eq!(
            unsafe { empty.install(Arc::new(RwLock::new(Count(count.clone())))) },
            Err(Error::Closing)
        );
        assert_eq!(count.load(Ordering::SeqCst), 3);
    }
    #[test]
    fn parent_and_child_slots_keep_shared_context_until_last_owner() {
        let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let parent = ContextSlot::default();
        let child = ContextSlot::default();
        unsafe {
            parent
                .install(Arc::new(RwLock::new(Count(count.clone()))))
                .unwrap();
            child.install(parent.borrow().unwrap()).unwrap();
            parent.destroy();
        }
        assert_eq!(count.load(Ordering::SeqCst), 0);
        assert!(unsafe { child.borrow() }.is_ok());
        unsafe {
            child.destroy();
        }
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }
}
