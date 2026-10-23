//! MightyMonitor's owned-lease endpoint. The legacy global Notify protocol is absent.
use crate::context::DeviceContext;
use mm_display_driver_protocol::{Mode, Request, Response, MAX_CLIENTS, MAX_FRAME, PIPE};
use std::{
    ptr::NonNull,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, LazyLock, Mutex, OnceLock,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::windows::named_pipe::{NamedPipeServer, ServerOptions},
    sync::Semaphore,
    time::timeout,
};
use wdf_umdf::{IddCxMonitorDeparture, WdfObjectDelete};
use wdf_umdf_sys::{IDDCX_ADAPTER__, IDDCX_MONITOR__};

pub static ADAPTER: OnceLock<AdapterObject> = OnceLock::new();
pub static MONITOR_MODES: LazyLock<Mutex<Vec<MonitorObject>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));
static MUTATIONS: Mutex<()> = Mutex::new(());
static NEXT_LEASE: AtomicU64 = AtomicU64::new(1);
static ORPHANS: Mutex<Vec<u32>> = Mutex::new(Vec::new());
#[derive(Debug)]
pub struct AdapterObject(pub NonNull<IDDCX_ADAPTER__>);
// SAFETY: handle belongs to WDF; mutations are serialized, not dereferenced here.
unsafe impl Send for AdapterObject {}
unsafe impl Sync for AdapterObject {}
#[derive(Debug)]
pub struct MonitorObject {
    pub object: Option<NonNull<IDDCX_MONITOR__>>,
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
            .get()
            .ok_or_else(|| anyhow::anyhow!("Adapter not ready"))?;
        let id = {
            let mut monitors = MONITOR_MODES
                .lock()
                .map_err(|_| anyhow::anyhow!("Driver state unavailable"))?;
            let id = (0..u32::from(crate::context::MAX_MONITORS))
                .find(|id| !monitors.iter().any(|m| m.data.id == *id))
                .ok_or_else(|| anyhow::anyhow!("No free display slots"))?;
            monitors.push(MonitorObject {
                object: None,
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
            DeviceContext::get_mut(adapter.0.as_ptr().cast(), |context| {
                result = context.create_monitor(id).map_err(Into::into);
            })
        };
        if context.is_err() {
            let _ = remove_monitor(id);
            return Err(anyhow::anyhow!("Adapter context unavailable"));
        }
        match result {
            Ok(identity) => Ok(Self {
                id,
                lease,
                identity,
                live: true,
            }),
            Err(error) => {
                if remove_monitor(id).is_err() {
                    if let Ok(mut orphans) = ORPHANS.lock() {
                        orphans.push(id);
                    }
                }
                Err(error)
            }
        }
    }
    fn stop(&mut self) -> anyhow::Result<()> {
        if self.live {
            let _mutation = MUTATIONS
                .lock()
                .map_err(|_| anyhow::anyhow!("Driver state unavailable"))?;
            remove_monitor(self.id)?;
            self.live = false;
        }
        Ok(())
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        if self.stop().is_err() {
            if let Ok(mut orphans) = ORPHANS.lock() {
                if !orphans.contains(&self.id) {
                    orphans.push(self.id);
                }
            }
        }
    }
}
fn remove_monitor(id: u32) -> anyhow::Result<()> {
    let record = {
        let mut monitors = MONITOR_MODES
            .lock()
            .map_err(|_| anyhow::anyhow!("Driver state unavailable"))?;
        let Some(index) = monitors.iter().position(|m| m.data.id == id) else {
            return Ok(());
        };
        monitors.remove(index)
    };
    if let Some(object) = record.object {
        // SAFETY: this exact live WDF monitor was removed from our registry under
        // MUTATIONS. No endpoint can delete it concurrently; no physical output
        // or another driver's monitor is enumerated/removed by this module.
        if record.arrived {
            if let Err(error) = unsafe { IddCxMonitorDeparture(object.as_ptr()) } {
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
async fn serve(mut pipe: NamedPipeServer) {
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
        let Ok(Ok(request)) = timeout(Duration::from_secs(45), request).await else {
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
            Request::Check { lease: id } if lease.as_ref().is_some_and(|l| l.lease == id) => {
                Response::Ok
            }
            Request::Remove { lease: id } if lease.as_ref().is_some_and(|l| l.lease == id) => {
                if lease.as_mut().is_some_and(|l| l.stop().is_ok()) {
                    lease = None;
                    Response::Ok
                } else {
                    Response::Rejected
                }
            }
            Request::Ping {} => Response::Ok,
            _ => Response::Rejected,
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
pub fn startup() {
    std::thread::spawn(|| {
        // Retry failed departures without making their connector slots reusable.
        std::thread::spawn(|| loop {
            std::thread::sleep(Duration::from_secs(1));
            let ids = if let Ok(mut orphans) = ORPHANS.lock() {
                std::mem::take(&mut *orphans)
            } else {
                continue;
            };
            for id in ids {
                let Ok(_mutation) = MUTATIONS.lock() else {
                    return;
                };
                if remove_monitor(id).is_err() {
                    if let Ok(mut orphans) = ORPHANS.lock() {
                        orphans.push(id);
                    }
                }
            }
        });
        let Ok(runtime) = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
        else {
            return;
        };
        runtime.block_on(async {
            use windows::{
                core::w,
                Win32::{
                    Foundation::{LocalFree, HLOCAL},
                    Security::{
                        Authorization::{
                            ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
                        },
                        PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES,
                    },
                },
            };
            let mut descriptor = PSECURITY_DESCRIPTOR::default();
            // SAFETY: fixed SDDL, valid exclusive output. SYSTEM/admins and local
            // interactive users only; no NULL DACL and remote clients are rejected.
            if unsafe {
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    w!("D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;IU)"),
                    SDDL_REVISION_1,
                    &mut descriptor,
                    None,
                )
            }
            .is_err()
            {
                return;
            }
            let mut attributes = SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: descriptor.0,
                bInheritHandle: false.into(),
            };
            let slots = Arc::new(Semaphore::new(MAX_CLIENTS));
            let mut first = true;
            loop {
                let Ok(permit) = slots.clone().acquire_owned().await else {
                    break;
                };
                // SAFETY: attributes/descriptor remain live throughout synchronous
                // creation. Windows copies the descriptor. The name is dedicated.
                let server = unsafe {
                    ServerOptions::new()
                        .first_pipe_instance(first)
                        .max_instances(MAX_CLIENTS)
                        .reject_remote_clients(true)
                        .in_buffer_size(512)
                        .out_buffer_size(512)
                        .create_with_security_attributes_raw(
                            PIPE,
                            std::ptr::addr_of_mut!(attributes).cast(),
                        )
                };
                let Ok(server) = server else {
                    break;
                };
                first = false;
                if server.connect().await.is_err() {
                    continue;
                }
                tokio::spawn(async move {
                    let _permit = permit;
                    serve(server).await;
                });
            }
            // SAFETY: descriptor is the allocation returned by the conversion API;
            // no pending create retains it after the synchronous call returns.
            unsafe {
                let _ = LocalFree(HLOCAL(descriptor.0));
            }
        });
    });
}
