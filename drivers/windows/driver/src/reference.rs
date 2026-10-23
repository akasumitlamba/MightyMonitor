//! One explicit WDF reference shared by Rust owners. Never install or trust a driver.
use wdf_umdf::{WdfError, WdfObjectDereference, WdfObjectReference};
use wdf_umdf_sys::WDFOBJECT;
#[derive(Debug)]
pub struct Reference(WDFOBJECT);
// SAFETY: WDF reference counting and the supported IDDCX calls are usable from
// these passive-level worker threads. Context access is separately synchronized.
unsafe impl Send for Reference {}
unsafe impl Sync for Reference {}
impl Reference {
    /// Caller holds a live framework callback/creation handle until reference acquired.
    pub unsafe fn new(object: WDFOBJECT) -> Result<Self, WdfError> {
        unsafe { WdfObjectReference(object)? };
        Ok(Self(object))
    }
    pub fn raw(&self) -> WDFOBJECT {
        self.0
    }
}
impl Drop for Reference {
    fn drop(&mut self) {
        // SAFETY: exactly one balanced native reference; context drops in EvtDestroy.
        unsafe {
            let _ = WdfObjectDereference(self.0);
        }
    }
}
