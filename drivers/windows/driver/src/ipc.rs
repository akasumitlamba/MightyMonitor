//! MightyMonitor's owned-lease endpoint. The legacy global Notify protocol is absent.
use crate::context::DeviceContext;
use mm_display_driver_protocol::{Mode, Request, Response, MAX_CLIENTS, MAX_FRAME, PIPE};
use std::{
    ptr::NonNull,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc, LazyLock, Mutex,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::windows::named_pipe::{NamedPipeServer, ServerOptions},
    sync::{watch, Semaphore},
    time::timeout,
};
use wdf_umdf::{IddCxMonitorDeparture, WdfObjectDelete};
use wdf_umdf_sys::{IDDCX_ADAPTER__, IDDCX_MONITOR__};

static ADAPTER: Mutex<Option<AdapterObject>> = Mutex::new(None);
static POWERED_DEVICE: AtomicUsize = AtomicUsize::new(0);
pub static MONITOR_MODES: LazyLock<Mutex<Vec<MonitorObject>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));
static MUTATIONS: Mutex<()> = Mutex::new(());
static NEXT_LEASE: AtomicU64 = AtomicU64::new(1);
static ORPHANS: Mutex<Vec<(u32, u64)>> = Mutex::new(Vec::new());
#[derive(Clone, Debug)]
struct AdapterObject {
    reference: Arc<crate::reference::Reference>,
    parent: usize,
}
#[derive(Debug)]
pub struct MonitorObject {
    pub object: Option<NonNull<IDDCX_MONITOR__>>,
    pub reference: Option<Arc<crate::reference::Reference>>,
    pub lease: u64,
    pub data: Monitor,
    pub arrived: bool,
}
// SAFETY: handle lifetime is owned by WDF and the record is protected by mutexes.
unsafe impl Send for MonitorObject {}
unsafe impl Sync for MonitorObject {}
#[derive(Debug)]
pub struct Monitor {
    pub id: u32,
    pub modes: Vec<MonitorMode>,
}
#[derive(Debug)]
pub struct MonitorMode {
    pub width: u32,
    pub height: u32,
    pub refresh_rates: Vec<u32>,
}
pub trait FlattenModes {
    fn flatten(&self) -> impl Iterator<Item = ModeItem>;
}
#[derive(Clone, Copy)]
pub struct ModeItem {
    pub width: u32,
    pub height: u32,
    pub refresh_rate: u32,
}
impl FlattenModes for Vec<MonitorMode> {
    fn flatten(&self) -> impl Iterator<Item = ModeItem> {
        self.iter().flat_map(|m| {
            m.refresh_rates.iter().map(|&hz| ModeItem {
                width: m.width,
                height: m.height,
                refresh_rate: hz,
            })
        })
    }
}
#[derive(Clone, Copy)]
pub struct NativeIdentity {
    pub adapter_luid: i64,
    pub target_id: u32,
}
struct Lease {
    id: u32,
    lease: u64,
    identity: NativeIdentity,
    live: bool,
}
impl Lease {
    fn create(mode: Mode) -> anyhow::Result<Self> {
        mode.validate()?;
        let _mutation = MUTATIONS
            .lock()
            .map_err(|_| anyhow::anyhow!("Driver state unavailable"))?;
        let lease = NEXT_LEASE
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| anyhow::anyhow!("Lease IDs exhausted"))?;
        let adapter = ADAPTER
            .lock()
            .map_err(|_| anyhow::anyhow!("Adapter unavailable"))?
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Adapter not ready"))?;
        if POWERED_DEVICE.load(Ordering::Acquire) != adapter.parent {
            return Err(anyhow::anyhow!("Adapter is powered down"));
        }
        let id = {
            let mut monitors = MONITOR_MODES
                .lock()
                .map_err(|_| anyhow::anyhow!("Driver state unavailable"))?;
            let id = (0..u32::from(crate::context::MAX_MONITORS))
                .find(|id| !monitors.iter().any(|m| m.data.id == *id))
                .ok_or_else(|| anyhow::anyhow!("No free display slots"))?;
            monitors.push(MonitorObject {
                object: None,
                reference: None,
                lease,
                arrived: false,
                data: Monitor {
                    id,
                    modes: vec![MonitorMode {
                        width: mode.width,
                        height: mode.height,
                        refresh_rates: vec![mode.refresh_hz],
                    }],
                },
            });
            id
        };
        let mut result = Err(anyhow::anyhow!("Adapter context unavailable"));
        // SAFETY: WDF owns the adapter handle. It was published only after init;
        // all endpoint create/remove operations are serialized by MUTATIONS.
        let context = unsafe {
            DeviceContext::get_mut(adapter.reference.raw().cast(), |context| {
                result = context.create_monitor(id).map_err(Into::into);
            })
        };
        if context.is_err() {
            let _ = remove_monitor(id, lease);
            return Err(anyhow::anyhow!("Adapter context unavailable"));
        }
        match result {
            Ok(identity) => {
                let present = MONITOR_MODES
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Driver state unavailable"))?
                    .iter()
                    .any(|m| m.data.id == id && m.lease == lease && m.reference.is_some());
                if !present {
                    return Err(anyhow::anyhow!("Display removed during creation"));
                }
                Ok(Self {
                    id,
                    lease,
                    identity,
                    live: true,
                })
            }
            Err(error) => {
                if remove_monitor(id, lease).is_err() {
                    if let Ok(mut orphans) = ORPHANS.lock() {
                        orphans.push((id, lease));
                    }
                }
                Err(error)
            }
        }
    }
    fn present(&self) -> bool {
        POWERED_DEVICE.load(Ordering::Acquire) != 0
            && MONITOR_MODES.lock().is_ok_and(|records| {
                records.iter().any(|m| {
                    m.data.id == self.id
                        && m.lease == self.lease
                        && m.arrived
                        && m.reference.is_some()
                })
            })
    }
    fn stop(&mut self) -> anyhow::Result<()> {
        if self.live {
            let _mutation = MUTATIONS
                .lock()
                .map_err(|_| anyhow::anyhow!("Driver state unavailable"))?;
            remove_monitor(self.id, self.lease)?;
            self.live = false;
        }
        Ok(())
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        if self.stop().is_err() {
            if let Ok(mut orphans) = ORPHANS.lock() {
                if !orphans.contains(&(self.id, self.lease)) {
                    orphans.push((self.id, self.lease));
                }
            }
        }
    }
}
fn remove_monitor(id: u32, lease: u64) -> anyhow::Result<()> {
    let record = {
        let mut monitors = MONITOR_MODES
            .lock()
            .map_err(|_| anyhow::anyhow!("Driver state unavailable"))?;
        let Some(index) = monitors
            .iter()
            .position(|m| m.data.id == id && m.lease == lease)
        else {
            return Ok(());
        };
        monitors.remove(index)
    };
    if let Some(object) = record.object {
        // The record owns a WDF reference; cleanup may mark closing but cannot
        // destroy this handle/context while the operation retains that reference.
        if unsafe { crate::context::MonitorContext::is_closing(object.as_ptr().cast()) } {
            return Ok(());
        }
        // SAFETY: this exact live WDF monitor was removed from our registry under
        // MUTATIONS. No endpoint can delete it concurrently; no physical output
        // or another driver's monitor is enumerated/removed by this module.
        if record.arrived {
            if let Err(error) = unsafe { IddCxMonitorDeparture(object.as_ptr()) } {
                if unsafe { crate::context::MonitorContext::is_closing(object.as_ptr().cast()) } {
                    return Ok(());
                }
                MONITOR_MODES
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Driver state unavailable"))?
                    .push(record);
                return Err(error.into());
            }
        } else {
            unsafe { WdfObjectDelete(object.as_ptr().cast()) };
        }
    }
    Ok(())
}
pub fn publish_adapter(object: NonNull<IDDCX_ADAPTER__>) -> Result<(), wdf_umdf::WdfError> {
    let reference = Arc::new(unsafe { crate::reference::Reference::new(object.as_ptr().cast())? });
    let mut parent = 0;
    unsafe { DeviceContext::get(object.as_ptr().cast(), |ctx| parent = ctx.parent_device())? };
    let mut registry = ADAPTER.lock().map_err(|_| wdf_umdf::WdfError::LockFailed)?;
    if registry.is_some() || unsafe { DeviceContext::is_closing(reference.raw()) } {
        return Err(wdf_umdf::WdfError::Unknown);
    }
    *registry = Some(AdapterObject { reference, parent });
    Ok(())
}
pub fn adapter_removed(object: wdf_umdf_sys::WDFOBJECT) {
    let removed = if let Ok(mut registry) = ADAPTER.lock() {
        if registry
            .as_ref()
            .is_some_and(|a| a.reference.raw() == object)
        {
            registry.take()
        } else {
            None
        }
    } else {
        None
    };
    drop(removed);
}
pub fn device_removed(device: wdf_umdf_sys::WDFOBJECT) {
    let _ =
        POWERED_DEVICE.compare_exchange(device as usize, 0, Ordering::AcqRel, Ordering::Acquire);
    let removed = if let Ok(mut registry) = ADAPTER.lock() {
        if registry
            .as_ref()
            .is_some_and(|a| a.parent == device as usize)
        {
            registry.take()
        } else {
            None
        }
    } else {
        None
    };
    drop(removed);
}
pub fn monitor_removed(object: wdf_umdf_sys::WDFOBJECT) {
    let removed = if let Ok(mut records) = MONITOR_MODES.lock() {
        records
            .iter()
            .position(|m| m.object.is_some_and(|p| p.as_ptr().cast() == object))
            .map(|i| records.remove(i))
    } else {
        None
    };
    if let Some(record) = removed {
        if let Ok(mut orphans) = ORPHANS.lock() {
            orphans.retain(|key| *key != (record.data.id, record.lease));
        }
        drop(record); // Last native reference is released outside the registry lock.
    }
}
pub fn power_up(device: wdf_umdf_sys::WDFDEVICE) {
    POWERED_DEVICE.store(device as usize, Ordering::Release);
}
pub fn power_down(device: wdf_umdf_sys::WDFDEVICE) {
    let Ok(_mutation) = MUTATIONS.lock() else {
        return;
    };
    let _ =
        POWERED_DEVICE.compare_exchange(device as usize, 0, Ordering::AcqRel, Ordering::Acquire);
}

async fn serve(mut pipe: NamedPipeServer, mut stop: watch::Receiver<bool>) {
    let mut lease: Option<Lease> = None;
    loop {
        let request = async {
            let size = usize::from(pipe.read_u16_le().await?);
            if size == 0 || size > MAX_FRAME {
                return Err(std::io::Error::other("Invalid frame"));
            }
            let mut bytes = vec![0; size];
            pipe.read_exact(&mut bytes).await?;
            mm_display_driver_protocol::decode_request(&bytes)
        };
        let result = tokio::select! {biased; _=stop.changed()=>break, result=timeout(Duration::from_secs(45),request)=>result};
        let Ok(Ok(request)) = result else {
            break;
        };
        let response = match request {
            Request::Create { mode } if lease.is_none() => {
                // If creation exceeds the response deadline, the detached native
                // task's eventual Lease result is dropped and rolls itself back.
                match timeout(
                    Duration::from_secs(10),
                    tokio::task::spawn_blocking(move || Lease::create(mode)),
                )
                .await
                {
                    Ok(Ok(Ok(created))) => {
                        let response = Response::Created {
                            lease: created.lease,
                            adapter_luid: created.identity.adapter_luid,
                            target_id: created.identity.target_id,
                        };
                        lease = Some(created);
                        response
                    }
                    _ => break,
                }
            }
            Request::Check { lease: id }
                if lease.as_ref().is_some_and(|l| l.lease == id && l.present()) =>
            {
                Response::Ok {}
            }
            Request::Remove { lease: id } if lease.as_ref().is_some_and(|l| l.lease == id) => {
                if lease.as_mut().is_some_and(|l| l.stop().is_ok()) {
                    lease = None;
                    Response::Ok {}
                } else {
                    Response::Rejected {}
                }
            }
            Request::Ping {} => Response::Ok {},
            _ => Response::Rejected {},
        };
        let Ok(frame) = mm_display_driver_protocol::encode(&response) else {
            break;
        };
        if !matches!(
            timeout(Duration::from_secs(3), pipe.write_all(&frame)).await,
            Ok(Ok(()))
        ) {
            break;
        }
    }
    // EOF, malformed/oversized input, idle/write timeout all drop this connection's
    // only Lease. There is no command to access another connection's lease.
}

struct Running {
    stop: watch::Sender<bool>,
    thread: std::thread::JoinHandle<()>,
}
static RUNNING: Mutex<Option<Running>> = Mutex::new(None);
static UNLOADING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub fn startup() {
    if UNLOADING.load(Ordering::Acquire) {
        return;
    }
    let Ok(mut running) = RUNNING.lock() else {
        return;
    };
    if running.is_some() {
        return;
    }
    let (stop, receiver) = watch::channel(false);
    if let Ok(thread) = std::thread::Builder::new()
        .name("mm-display-endpoint".into())
        .spawn(move || run(receiver))
    {
        *running = Some(Running { stop, thread });
    }
}
/// Called by EvtDriverUnload: no Rust thread may execute code from an unloaded DLL.
/// Never forcibly terminate a native framework call or detach its worker thread.
pub fn shutdown() {
    UNLOADING.store(true, Ordering::Release);
    let running = RUNNING.lock().ok().and_then(|mut running| running.take());
    if let Some(running) = running {
        running.stop.send_replace(true);
        let _ = running.thread.join();
    }
}
async fn reap(mut stop: watch::Receiver<bool>) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    loop {
        tokio::select! {biased; _=stop.changed()=>break, _=interval.tick()=>{}}
        let ids = if let Ok(mut orphans) = ORPHANS.lock() {
            std::mem::take(&mut *orphans)
        } else {
            continue;
        };
        for (id, lease) in ids {
            let Ok(_mutation) = MUTATIONS.lock() else {
                return;
            };
            if remove_monitor(id, lease).is_err() {
                if let Ok(mut orphans) = ORPHANS.lock() {
                    orphans.push((id, lease));
                }
            }
        }
    }
}
fn run(mut stop: watch::Receiver<bool>) {
    let Ok(runtime) = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
    else {
        return;
    };
    runtime.block_on(async {
        use windows::{core::w,Win32::{Security::{Authorization::{ConvertStringSecurityDescriptorToSecurityDescriptorW,SDDL_REVISION_1},PSECURITY_DESCRIPTOR,SECURITY_ATTRIBUTES},Foundation::{LocalFree,HLOCAL}}};
        struct Descriptor(PSECURITY_DESCRIPTOR);
        impl Drop for Descriptor {fn drop(&mut self){
            // SAFETY: exactly the allocation returned by the conversion API.
            unsafe{let _=LocalFree(HLOCAL(self.0.0));}
        }}
        let mut descriptor=PSECURITY_DESCRIPTOR::default();
        // SAFETY: constant local-only ACL and valid exclusive output pointer.
        if unsafe{ConvertStringSecurityDescriptorToSecurityDescriptorW(w!("D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;IU)"),SDDL_REVISION_1,&mut descriptor,None)}.is_err(){return;}
        let descriptor=Descriptor(descriptor);
        let mut attributes=SECURITY_ATTRIBUTES{nLength:std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,lpSecurityDescriptor:descriptor.0.0,bInheritHandle:false.into()};
        let slots=Arc::new(Semaphore::new(MAX_CLIENTS));let mut first=true;
        let _reaper=tokio::spawn(reap(stop.clone()));
        loop {
            let permit=tokio::select!{biased; _=stop.changed()=>break, result=slots.clone().acquire_owned()=>match result {Ok(permit)=>permit,Err(_)=>break}};
            // SAFETY: descriptor and attributes remain live until the synchronous
            // create finishes. The OS copies them; no task captures raw pointers.
            let server=unsafe{ServerOptions::new().first_pipe_instance(first).max_instances(MAX_CLIENTS).reject_remote_clients(true).in_buffer_size(512).out_buffer_size(512).create_with_security_attributes_raw(PIPE,std::ptr::addr_of_mut!(attributes).cast())};
            let Ok(server)=server else{break;};first=false;
            let connected=tokio::select!{biased; _=stop.changed()=>break, result=server.connect()=>result};
            if connected.is_err(){continue;}
            let cancelled=stop.clone();
            tokio::spawn(async move{let _permit=permit;serve(server,cancelled).await;});
        }
    });
    // Runtime drop aborts async work and joins its threads, including any late
    // native creation task and that task's owned rollback, before DLL unload.
}
