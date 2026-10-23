//! Client for the dedicated MightyMonitor UMDF driver. Does not install anything.
use crate::{
    owned::{Backend, CreatedOutput, Mode, OutputId},
    Display,
};
use mm_display_driver_protocol::{Request, Response, MAX_FRAME, PIPE};
use std::{io, os::windows::io::AsRawHandle, sync::mpsc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::windows::named_pipe::{ClientOptions, NamedPipeClient},
    runtime::Runtime,
};

struct Job {
    request: Request,
    reply: mpsc::SyncSender<io::Result<Response>>,
}
struct Connection {
    send: Option<mpsc::SyncSender<Job>>,
}
fn unavailable() -> io::Error {
    io::Error::other(
        "MightyMonitor display driver is unavailable. No display was selected instead.",
    )
}
impl Connection {
    fn open() -> io::Result<Self> {
        let (send, receive) = mpsc::sync_channel::<Job>(1);
        let (ready_send, ready_receive) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("mm-display-driver".into())
            .spawn(move || {
                let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                else {
                    let _ = ready_send.send(Err(unavailable()));
                    return;
                };
                let opened = runtime.block_on(async {
                    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
                    loop {
                        // Identification only: a pipe owner cannot impersonate this user.
                        match ClientOptions::new()
                            .security_qos_flags(0x0001_0000)
                            .open(PIPE)
                        {
                            Ok(pipe) => {
                                authenticate(&pipe)?;
                                break Ok(pipe);
                            }
                            Err(error)
                                if error.raw_os_error() == Some(231)
                                    && tokio::time::Instant::now() < deadline =>
                            {
                                tokio::time::sleep(Duration::from_millis(25)).await
                            }
                            Err(_) => break Err(unavailable()),
                        }
                    }
                });
                let mut pipe = match opened {
                    Ok(pipe) => pipe,
                    Err(error) => {
                        let _ = ready_send.send(Err(error));
                        return;
                    }
                };
                if !matches!(
                    exchange(&runtime, &mut pipe, Request::Ping {}),
                    Ok(Response::Ok {})
                ) {
                    let _ = ready_send.send(Err(unavailable()));
                    return;
                }
                if ready_send.send(Ok(())).is_err() {
                    return;
                }
                loop {
                    match receive.recv_timeout(Duration::from_secs(15)) {
                        Ok(job) => {
                            let response = exchange(&runtime, &mut pipe, job.request);
                            let broken = response.is_err();
                            let _ = job.reply.send(response);
                            if broken {
                                break;
                            }
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            if !matches!(
                                exchange(&runtime, &mut pipe, Request::Ping {}),
                                Ok(Response::Ok {})
                            ) {
                                break;
                            }
                        }
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
                // Closing this unique connection triggers driver-side owned cleanup.
            })?;
        ready_receive
            .recv_timeout(Duration::from_secs(7))
            .map_err(|_| unavailable())??;
        Ok(Self { send: Some(send) })
    }
    fn call(&mut self, request: Request) -> io::Result<Response> {
        let wait = if matches!(&request, Request::Create { .. }) {
            14
        } else {
            5
        };
        let (reply, receive) = mpsc::sync_channel(1);
        let result = (|| {
            self.send
                .as_ref()
                .ok_or_else(unavailable)?
                .try_send(Job { request, reply })
                .map_err(|_| unavailable())?;
            receive
                .recv_timeout(Duration::from_secs(wait))
                .map_err(|_| unavailable())?
        })();
        if result.is_err() {
            self.send = None;
        }
        result
    }
}
fn exchange(
    runtime: &Runtime,
    pipe: &mut NamedPipeClient,
    request: Request,
) -> io::Result<Response> {
    let wait = if matches!(&request, Request::Create { .. }) {
        12
    } else {
        3
    };
    let frame = mm_display_driver_protocol::encode(&request)?;
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(wait), async {
            pipe.write_all(&frame).await?;
            let size = usize::from(pipe.read_u16_le().await?);
            if size == 0 || size > MAX_FRAME {
                return Err(io::Error::other("Invalid driver response"));
            }
            let mut payload = vec![0; size];
            pipe.read_exact(&mut payload).await?;
            mm_display_driver_protocol::decode_response(&payload)
        })
        .await
        .map_err(|_| unavailable())?
    })
}
struct Live {
    created: CreatedOutput,
    mode: Mode,
    display: Option<Display>,
}
/// Requires an already installed, trusted MightyMonitor driver. All pipe I/O has
/// deadlines and runs on a dedicated runtime thread, including from async hosts.
/// Creation failures close the connection if explicit rollback cannot finish.
pub struct DriverBackend {
    connection: Connection,
    live: Option<Live>,
}
impl DriverBackend {
    pub fn connect() -> io::Result<Self> {
        Ok(Self {
            connection: Connection::open()?,
            live: None,
        })
    }
    pub(crate) fn display(&self) -> io::Result<Display> {
        self.live
            .as_ref()
            .and_then(|l| l.display.clone())
            .ok_or_else(unavailable)
    }
}
impl Backend for DriverBackend {
    fn create(&mut self, mode: Mode) -> io::Result<CreatedOutput> {
        if self.live.is_some() {
            return Err(io::Error::other(
                "This driver connection already owns a display",
            ));
        }
        let request_mode = mm_display_driver_protocol::Mode {
            width: mode.width,
            height: mode.height,
            refresh_hz: mode.refresh_hz,
        };
        request_mode.validate()?;
        let Response::Created {
            lease,
            adapter_luid,
            target_id,
        } = self
            .connection
            .call(Request::Create { mode: request_mode })?
        else {
            return Err(io::Error::other("Display creation was rejected"));
        };
        let created = CreatedOutput {
            lease_id: lease,
            output: OutputId {
                adapter_luid,
                target_id,
            },
        };
        self.live = Some(Live {
            created: CreatedOutput {
                lease_id: lease,
                output: created.output,
            },
            mode,
            display: None,
        });
        let resolved = (|| {
            if lease == 0 {
                return Err(io::Error::other("Invalid driver lease"));
            }
            // Windows may publish an arrived output after the driver responds.
            // Wait only for this exact output; do not activate/rearrange others.
            let until = std::time::Instant::now() + Duration::from_secs(3);
            let display = loop {
                match crate::resolve_output(created.output) {
                    Ok(display) => break display,
                    Err(_) if std::time::Instant::now() < until => {
                        std::thread::sleep(Duration::from_millis(25))
                    }
                    Err(error) => return Err(error),
                }
            };
            if display.width != mode.width || display.height != mode.height {
                return Err(io::Error::other("Created display has an unexpected size"));
            }
            display.validate()?;
            Ok(display)
        })();
        match resolved {
            Ok(display) => {
                self.live.as_mut().expect("armed output").display = Some(display);
                Ok(created)
            }
            Err(error) => {
                if !matches!(
                    self.connection.call(Request::Remove { lease }),
                    Ok(Response::Ok {})
                ) {
                    self.connection.send = None;
                }
                self.live = None;
                Err(error)
            }
        }
    }
    fn validate(&mut self, created: &CreatedOutput, mode: Mode) -> io::Result<()> {
        let live = self.live.as_ref().ok_or_else(unavailable)?;
        if &live.created != created || live.mode != mode {
            return Err(io::Error::other("Driver lease identity changed"));
        }
        if !matches!(
            self.connection.call(Request::Check {
                lease: created.lease_id
            })?,
            Response::Ok {}
        ) {
            return Err(unavailable());
        }
        let current = crate::resolve_output(created.output)?;
        if live.display.as_ref() != Some(&current) {
            return Err(io::Error::other("Owned display changed or disconnected"));
        }
        current.validate()
    }
    fn remove(&mut self, created: &CreatedOutput) -> io::Result<()> {
        let Some(live) = self.live.as_ref() else {
            return Ok(());
        };
        if &live.created != created {
            return Err(io::Error::other("Cannot remove a foreign display lease"));
        }
        if !matches!(
            self.connection.call(Request::Remove {
                lease: created.lease_id
            })?,
            Response::Ok {}
        ) {
            return Err(io::Error::other("Could not remove owned display"));
        }
        self.live = None;
        Ok(())
    }
}

fn authenticate(pipe: &NamedPipeClient) -> io::Result<()> {
    use windows::{
        core::PWSTR,
        Win32::{
            Foundation::{CloseHandle, HANDLE},
            Security::{
                GetTokenInformation, IsWellKnownSid, TokenUser, WinLocalServiceSid,
                WinLocalSystemSid, TOKEN_QUERY, TOKEN_USER,
            },
            System::{
                Pipes::GetNamedPipeServerProcessId,
                SystemInformation::GetSystemDirectoryW,
                Threading::{
                    OpenProcess, OpenProcessToken, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
                    PROCESS_QUERY_LIMITED_INFORMATION,
                },
            },
        },
    };
    struct OwnedHandle(HANDLE);
    impl Drop for OwnedHandle {
        fn drop(&mut self) {
            // SAFETY: handle was obtained by this function and is closed exactly once.
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
    let mut pid = 0;
    // SAFETY: Tokio owns a live local pipe; output points to initialized storage.
    unsafe { GetNamedPipeServerProcessId(HANDLE(pipe.as_raw_handle()), &mut pid) }
        .map_err(|_| unavailable())?;
    // SAFETY: read-only query rights; the returned handle is immediately owned.
    let process = OwnedHandle(
        unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }
            .map_err(|_| unavailable())?,
    );
    let mut token = HANDLE::default();
    // SAFETY: live process, read-only token access, valid output.
    unsafe { OpenProcessToken(process.0, TOKEN_QUERY, &mut token) }.map_err(|_| unavailable())?;
    let token = OwnedHandle(token);
    let mut buffer = [0usize; 128];
    let mut size = 0;
    // SAFETY: aligned initialized buffer advertised at its full byte size, valid token.
    unsafe {
        GetTokenInformation(
            token.0,
            TokenUser,
            Some(buffer.as_mut_ptr().cast()),
            std::mem::size_of_val(&buffer) as u32,
            &mut size,
        )
    }
    .map_err(|_| unavailable())?;
    if size < (std::mem::size_of::<TOKEN_USER>() as u32)
        || size as usize > std::mem::size_of_val(&buffer)
    {
        return Err(unavailable());
    }
    // SAFETY: TOKEN_USER is initialized by the successful kernel token query and
    // remains backed by the aligned buffer throughout both read-only SID checks.
    let user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
    let trusted = unsafe {
        IsWellKnownSid(user.User.Sid, WinLocalServiceSid).as_bool()
            || IsWellKnownSid(user.User.Sid, WinLocalSystemSid).as_bool()
    };
    if !trusted {
        return Err(unavailable());
    }
    let mut image = [0u16; 1024];
    let mut length = image.len() as u32;
    // SAFETY: bounded output buffer and size, live query-only process handle.
    unsafe {
        QueryFullProcessImageNameW(
            process.0,
            PROCESS_NAME_WIN32,
            PWSTR(image.as_mut_ptr()),
            &mut length,
        )
    }
    .map_err(|_| unavailable())?;
    let mut system = [0u16; 1024];
    // SAFETY: fixed-size mutable output slice, no retained pointers.
    let system_length = unsafe { GetSystemDirectoryW(Some(&mut system)) } as usize;
    if length == 0
        || length as usize > image.len()
        || system_length == 0
        || system_length >= system.len()
    {
        return Err(unavailable());
    }
    let image = String::from_utf16(&image[..length as usize]).map_err(|_| unavailable())?;
    let expected = format!(
        "{}\\WUDFHost.exe",
        String::from_utf16(&system[..system_length]).map_err(|_| unavailable())?
    );
    if !image.eq_ignore_ascii_case(&expected) {
        return Err(unavailable());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
    fn pair(runtime: &Runtime) -> (NamedPipeServer, NamedPipeClient) {
        runtime.block_on(async {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let name = format!(
                r"\\.\pipe\MightyMonitor.Test.{}.{nonce}",
                std::process::id()
            );
            let server = ServerOptions::new()
                .first_pipe_instance(true)
                .reject_remote_clients(true)
                .create(&name)
                .unwrap();
            let client = ClientOptions::new().open(&name).unwrap();
            server.connect().await.unwrap();
            (server, client)
        })
    }
    #[test]
    fn rejects_a_local_application_impersonating_the_driver() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let (_server, client) = pair(&runtime);
        assert!(authenticate(&client).is_err());
    }
    #[test]
    fn oversized_pipe_reply_is_rejected_without_reading_its_body() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let (mut server, mut client) = pair(&runtime);
        let task = runtime.spawn(async move {
            let size = server.read_u16_le().await.unwrap();
            let mut request = vec![0; usize::from(size)];
            server.read_exact(&mut request).await.unwrap();
            server.write_u16_le((MAX_FRAME + 1) as u16).await.unwrap();
        });
        assert!(exchange(&runtime, &mut client, Request::Ping {}).is_err());
        runtime.block_on(task).unwrap();
    }
    #[test]
    fn stalled_driver_reply_has_a_deadline() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let (_server, mut client) = pair(&runtime);
        let start = std::time::Instant::now();
        assert!(exchange(&runtime, &mut client, Request::Ping {}).is_err());
        assert!(start.elapsed() < Duration::from_secs(6));
    }
}
