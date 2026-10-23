#![allow(non_snake_case)]
#![allow(clippy::missing_errors_doc)]

use std::ffi::c_void;
use std::sync::OnceLock;

use wdf_umdf_sys::{
    _WDF_DEVICE_PROPERTY_DATA, _WDF_PNPPOWER_EVENT_CALLBACKS, DEVPROPTYPE, NTSTATUS,
    PCUNICODE_STRING, PCWDF_OBJECT_CONTEXT_TYPE_INFO, PDRIVER_OBJECT, POOL_TYPE, PWDFDEVICE_INIT,
    PWDF_DRIVER_CONFIG, PWDF_OBJECT_ATTRIBUTES, WDFDEVICE, WDFDRIVER, WDFMEMORY, WDFOBJECT,
    WDF_DEVICE_FAILED_ACTION, WDF_NO_HANDLE, WDF_NO_OBJECT_ATTRIBUTES, WDF_OBJECT_ATTRIBUTES,
};

#[derive(Copy, Clone, Debug, thiserror::Error)]
pub enum WdfError {
    #[error("{0}")]
    WdfFunctionNotAvailable(&'static str),
    #[error("{0}")]
    CallFailed(NTSTATUS),
    #[error("Failed to upgrade Arc pointer")]
    UpgradeFailed,
    #[error("Failed to lock")]
    LockFailed,
    #[error("Unknown")]
    Unknown,
    // this is required for success status for ()
    #[error("This is not an error, ignore it")]
    _Success,
}

impl From<()> for WdfError {
    fn from(_: ()) -> Self {
        WdfError::_Success
    }
}

impl From<*mut c_void> for WdfError {
    fn from(_: *mut c_void) -> Self {
        Self::Unknown
    }
}

impl From<WdfError> for NTSTATUS {
    fn from(value: WdfError) -> Self {
        #[allow(clippy::enum_glob_use)]
        use WdfError::*;
        match value {
            WdfFunctionNotAvailable(_) => Self::STATUS_NOT_FOUND,
            CallFailed(status) => status,
            UpgradeFailed => Self::STATUS_INVALID_HANDLE,
            LockFailed => Self::STATUS_WAS_LOCKED,
            Unknown => Self::STATUS_DRIVER_INTERNAL_ERROR,
            _Success => 0.into(),
        }
    }
}

impl From<NTSTATUS> for WdfError {
    fn from(value: NTSTATUS) -> Self {
        WdfError::CallFailed(value)
    }
}

macro_rules! WdfCall {
    ($name:ident ( $($args:expr),* )) => {
        WdfCall!(false, $name($($args),*))
    };

    ($other_is_error:expr, $name:ident ( $($args:expr),* )) => {{
        static CACHED_FN: OnceLock<
            Result<
                ::paste::paste!(::wdf_umdf_sys::[<PFN_ $name:upper>]),
                WdfError
            >
        > = OnceLock::new();

        let f = CACHED_FN.get_or_init(|| {
            ::paste::paste! {
                const FN_INDEX: usize = ::wdf_umdf_sys::WDFFUNCENUM::[<$name TableIndex>].0 as usize;

                // validate that wdf function can be used
                let is_available = ::wdf_umdf_sys::WdfIsFunctionAvailable!($name);

                if is_available {
                    // SAFETY: Only immutable accesses are done to this
                    let fn_table = unsafe { ::wdf_umdf_sys::WdfFunctions_02031 };

                    // SAFETY: Read-only, initialized by the time we use it, and checked to be in bounds
                    let f = unsafe {
                        fn_table
                        .add(FN_INDEX)
                        .cast::<::wdf_umdf_sys::[<PFN_ $name:upper>]>()
                    };

                    // SAFETY: Ensured that this is present by if condition from `WdfIsFunctionAvailable!`
                    let f = unsafe { f.read() };

                    Ok(f)
                } else {
                    Err($crate::WdfError::WdfFunctionNotAvailable(concat!(stringify!($name), " is not available")))
                }
            }
        }).clone()?;

        // SAFETY: Above: If it's Ok, then it's guaranteed to be Some(fn)
        let f = unsafe { f.unwrap_unchecked() };

        // SAFETY: Pointer to globals is always immutable
        let globals = unsafe { ::wdf_umdf_sys::WdfDriverGlobals };

        // SAFETY: None. User is responsible for safety and must use their own unsafe block
        let result = unsafe { f(globals, $($args),*) };

        if $crate::is_nt_error(&result, $other_is_error) {
            Err(result.into())
        } else {
            Ok(result.into())
        }

    }};
}

/// Unlike the official `WDF_DECLARE_CONTEXT_TYPE` macro, you only need to declare this on the actual data struct want to use
/// Safety is maintained through a `RwLock` of the underlying data
///
/// This generates associated fns `init`/`get`/`drop`/`get_type_info` on your `$context_type` with the same visibility
///
/// Example:
/// ```rust
/// pub struct IndirectDeviceContext {
///     device: WDFDEVICE,
/// }
///
/// impl IndirectDeviceContext {
///     pub fn new(device: WDFDEVICE) -> Self {
///         Self { device }
///     }
/// }
///
/// WDF_DECLARE_CONTEXT_TYPE!(pub IndirectDeviceContext);
///
/// // with a `device: WDFDEVICE`
/// let context = IndirectDeviceContext::new(device as WDFOBJECT);
/// IndirectDeviceContext::init(context);
/// // elsewhere
/// let mutable_access = IndirectDeviceContext::get_mut(device).unwrap();
/// ```
#[macro_export]
macro_rules! WDF_DECLARE_CONTEXT_TYPE {
    ($sv:vis $context_type:ident) => {
        $crate::paste! {
            #[allow(non_snake_case)]
            mod [<WdfObject $context_type>] {
                use super::$context_type;
                #[repr(transparent)]
                struct TypeInfo<T> {
                    cell: ::std::cell::UnsafeCell<$crate::wdf_umdf_sys::_WDF_OBJECT_CONTEXT_TYPE_INFO>,
                    marker: ::std::marker::PhantomData<T>,
                }
                // SAFETY: immutable type metadata, independent of context instances.
                unsafe impl<T: Sync> Sync for TypeInfo<T> {}
                static TYPE_INFO: TypeInfo<$context_type> = TypeInfo {
                    cell: ::std::cell::UnsafeCell::new($crate::wdf_umdf_sys::_WDF_OBJECT_CONTEXT_TYPE_INFO {
                        Size: ::std::mem::size_of::<$crate::wdf_umdf_sys::_WDF_OBJECT_CONTEXT_TYPE_INFO>() as u32,
                        ContextName: concat!(stringify!($context_type), "\0").as_ptr().cast(),
                        ContextSize: ::std::mem::size_of::<$crate::ContextSlot<$context_type>>(),
                        UniqueType: &TYPE_INFO as *const _ as *const _,
                        EvtDriverGetUniqueContextType: None,
                    }),
                    marker: ::std::marker::PhantomData,
                };
                impl $context_type {
                    /// Caller pins this live WDF object through the returned borrow.
                    unsafe fn slot<'a>(handle: $crate::wdf_umdf_sys::WDFOBJECT) -> Result<&'a $crate::ContextSlot<Self>, $crate::WdfError> {
                        let ptr=unsafe{$crate::WdfObjectGetTypedContextWorker(handle,TYPE_INFO.cell.get())?}.cast::<$crate::ContextSlot<Self>>();
                        let ptr=::std::ptr::NonNull::new(ptr).ok_or($crate::WdfError::Unknown)?;
                        // SAFETY: WDF allocated zeroed, aligned context of ContextSize.
                        Ok(unsafe{ptr.as_ref()})
                    }
                    unsafe fn arc(handle:$crate::wdf_umdf_sys::WDFOBJECT)->Result<::std::sync::Arc<::std::sync::RwLock<Self>>,$crate::WdfError> {
                        unsafe{Self::slot(handle)?.borrow()}.map_err(|_|$crate::WdfError::UpgradeFailed)
                    }
                    /// Pin the newly created WDF object before initializing it.
                    $sv unsafe fn init(self,handle:$crate::wdf_umdf_sys::WDFOBJECT)->Result<(),$crate::WdfError> {
                        unsafe{Self::slot(handle)?.install(::std::sync::Arc::new(::std::sync::RwLock::new(self)))}.map_err(|_|$crate::WdfError::Unknown)
                    }
                    /// Both source and fresh destination objects must be pinned.
                    $sv unsafe fn clone_into(&self,handle:$crate::wdf_umdf_sys::WDFOBJECT)->Result<(),$crate::WdfError> {
                        let value=unsafe{Self::arc(self.device.cast())?};
                        unsafe{Self::slot(handle)?.install(value)}.map_err(|_|$crate::WdfError::Unknown)
                    }
                    /// Cleanup closes access; it does not free context still in use.
                    $sv unsafe fn mark_closing(handle:$crate::wdf_umdf_sys::WDFOBJECT)->Result<(),$crate::WdfError> {
                        unsafe{Self::slot(handle)?}.mark_closing();Ok(())
                    }
                    $sv unsafe fn is_closing(handle:$crate::wdf_umdf_sys::WDFOBJECT)->bool {
                        unsafe{Self::slot(handle)}.map_or(true,|slot|slot.is_closing())
                    }
                    /// EvtCleanup only; reject new calls before releasing owned resources.
                    $sv unsafe fn cleanup_mut<F:FnOnce(&mut Self)>(handle:$crate::wdf_umdf_sys::WDFOBJECT,cb:F)->Result<(),$crate::WdfError> {
                        let slot=unsafe{Self::slot(handle)?};slot.mark_closing();
                        let arc=unsafe{slot.borrow_during_cleanup()}.map_err(|_|$crate::WdfError::UpgradeFailed)?;
                        let mut guard=arc.write().map_err(|_|$crate::WdfError::LockFailed)?;cb(&mut guard);Ok(())
                    }
                    /// EvtDestroy only. No native object references remain.
                    $sv unsafe fn drop(handle:$crate::wdf_umdf_sys::WDFOBJECT)->Result<(),$crate::WdfError> {
                        unsafe{Self::slot(handle)?.destroy()};Ok(())
                    }
                    $sv unsafe fn get<F:FnOnce(&Self)>(handle:*mut $crate::wdf_umdf_sys::WDFDEVICE__,cb:F)->Result<(),$crate::WdfError> {
                        let arc=unsafe{Self::arc(handle.cast())?};let guard=arc.read().map_err(|_|$crate::WdfError::LockFailed)?;cb(&guard);Ok(())
                    }
                    $sv unsafe fn get_mut<F:FnOnce(&mut Self)>(handle:*mut $crate::wdf_umdf_sys::WDFDEVICE__,cb:F)->Result<(),$crate::WdfError> {
                        let arc=unsafe{Self::arc(handle.cast())?};let mut guard=arc.write().map_err(|_|$crate::WdfError::LockFailed)?;cb(&mut guard);Ok(())
                    }
                    $sv unsafe fn try_get<F:FnOnce(&Self)>(handle:*mut $crate::wdf_umdf_sys::WDFDEVICE__,cb:F)->Result<(),$crate::WdfError> {
                        let arc=unsafe{Self::arc(handle.cast())?};let guard=arc.try_read().map_err(|_|$crate::WdfError::LockFailed)?;cb(&guard);Ok(())
                    }
                    $sv unsafe fn try_get_mut<F:FnOnce(&mut Self)>(handle:*mut $crate::wdf_umdf_sys::WDFDEVICE__,cb:F)->Result<(),$crate::WdfError> {
                        let arc=unsafe{Self::arc(handle.cast())?};let mut guard=arc.try_write().map_err(|_|$crate::WdfError::LockFailed)?;cb(&mut guard);Ok(())
                    }
                    $sv unsafe fn get_type_info()->&'static $crate::wdf_umdf_sys::_WDF_OBJECT_CONTEXT_TYPE_INFO {
                        unsafe{&*TYPE_INFO.cell.get()}
                    }
                }
            }
        }
    };
}

/// # Safety
///
/// None. User is responsible for safety.
pub unsafe fn WdfDriverCreate(
    // in
    DriverObject: PDRIVER_OBJECT,
    // in
    RegistryPath: PCUNICODE_STRING,
    // in, optional
    DriverAttributes: Option<PWDF_OBJECT_ATTRIBUTES>,
    // in
    DriverConfig: PWDF_DRIVER_CONFIG,
    // out, optional
    Driver: Option<&mut WDFDRIVER>,
) -> Result<NTSTATUS, WdfError> {
    WdfCall! {
        WdfDriverCreate(
            DriverObject,
            RegistryPath,
            DriverAttributes.unwrap_or(WDF_NO_OBJECT_ATTRIBUTES!()),
            DriverConfig,
            Driver
                .map(std::ptr::from_mut)
                .unwrap_or(WDF_NO_HANDLE!())
        )
    }
}

/// # Safety
///
/// None. User is responsible for safety.
pub unsafe fn WdfDeviceCreate(
    // in, out
    DeviceInit: &mut PWDFDEVICE_INIT,
    // in, optional
    DeviceAttributes: Option<&mut WDF_OBJECT_ATTRIBUTES>,
    // out
    Device: &mut WDFDEVICE,
) -> Result<NTSTATUS, WdfError> {
    WdfCall! {
        WdfDeviceCreate(
            DeviceInit,
            DeviceAttributes.map_or(WDF_NO_OBJECT_ATTRIBUTES!(), std::ptr::from_mut),
            Device
        )
    }
}

/// # Safety
///
/// None. User is responsible for safety.
pub unsafe fn WdfDeviceInitSetPnpPowerEventCallbacks(
    // in
    DeviceInit: PWDFDEVICE_INIT,
    // in
    PnpPowerEventCallbacks: *mut _WDF_PNPPOWER_EVENT_CALLBACKS,
) -> Result<(), WdfError> {
    WdfCall! {
        WdfDeviceInitSetPnpPowerEventCallbacks(
            DeviceInit,
            PnpPowerEventCallbacks
        )
    }
}

/// # Safety
///
/// None. User is responsible for safety.
pub unsafe fn WdfObjectGetTypedContextWorker(
    // in
    Handle: WDFOBJECT,
    // in
    TypeInfo: PCWDF_OBJECT_CONTEXT_TYPE_INFO,
) -> Result<*mut c_void, WdfError> {
    WdfCall! {
        WdfObjectGetTypedContextWorker(
            Handle,
            TypeInfo
        )
    }
}

/// # Safety
///
/// None. User is responsible for safety.
pub unsafe fn WdfObjectDelete(
    // in
    Object: WDFOBJECT,
) -> Result<(), WdfError> {
    WdfCall! {
        WdfObjectDelete(
            Object
        )
    }
}

/// # Safety
///
/// None. User is responsible for safety.
pub unsafe fn WdfDeviceSetFailed(
    // in
    Device: WDFDEVICE,
    // in
    FailedAction: WDF_DEVICE_FAILED_ACTION,
) -> Result<(), WdfError> {
    WdfCall! {
        WdfDeviceSetFailed(
            Device,
            FailedAction
        )
    }
}

/// # Safety
///
/// None. User is responsible for safety.
pub unsafe fn WdfDeviceAllocAndQueryPropertyEx(
    // in
    Device: WDFDEVICE,
    // in
    DeviceProperty: &mut _WDF_DEVICE_PROPERTY_DATA,
    // in
    PoolType: POOL_TYPE,
    // in, optional
    PropertyMemoryAttributes: PWDF_OBJECT_ATTRIBUTES,
    // out
    PropertyMemory: &mut WDFMEMORY,
    // out
    Type: &mut DEVPROPTYPE,
) -> Result<NTSTATUS, WdfError> {
    WdfCall! {
        WdfDeviceAllocAndQueryPropertyEx(
            Device,
            DeviceProperty,
            PoolType,
            PropertyMemoryAttributes,
            PropertyMemory,
            Type
        )
    }
}

/// # Safety
///
/// None. User is responsible for safety.
pub unsafe fn WdfMemoryGetBuffer(
    // in
    Memory: WDFMEMORY,
    // out, optional
    BufferSize: Option<&mut usize>,
) -> Result<*mut c_void, WdfError> {
    WdfCall! {
        WdfMemoryGetBuffer(
            Memory,
            BufferSize.map_or(std::ptr::null_mut(), std::ptr::from_mut)
        )
    }
}

/// Caller owns a live framework handle; balances with Dereference after use.
pub unsafe fn WdfObjectReference(Object: WDFOBJECT) -> Result<(), WdfError> {
    WdfCall! { WdfObjectReferenceActual(Object, std::ptr::null_mut(), 0, std::ptr::null()) }
}
/// Caller owns exactly one reference obtained by WdfObjectReference.
pub unsafe fn WdfObjectDereference(Object: WDFOBJECT) -> Result<(), WdfError> {
    WdfCall! { WdfObjectDereferenceActual(Object, std::ptr::null_mut(), 0, std::ptr::null()) }
}
