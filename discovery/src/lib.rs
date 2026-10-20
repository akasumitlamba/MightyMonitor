//! Untrusted local presence only. Discovery never carries a viewing credential.
#![forbid(unsafe_code)]
use mdns_sd::{IfKind, ResolvedService, ServiceDaemon, ServiceEvent, ServiceInfo};
use std::{
    collections::BTreeMap,
    io,
    net::{IpAddr, SocketAddr},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
pub const SERVICE_TYPE: &str = "_mightymonitor._tcp.local.";
const MAX_PEERS: usize = 32;
fn error(e: impl std::fmt::Display) -> io::Error {
    io::Error::other(e.to_string())
}
fn routable(address: IpAddr) -> bool {
    !address.is_unspecified() && !address.is_multicast() && !address.is_loopback()
}
fn service(address: SocketAddr, platform: &str, extend: bool) -> io::Result<ServiceInfo> {
    if address.port() == 0
        || !routable(address.ip())
        || !["windows", "linux", "android"].contains(&platform)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Discovery requires a specific LAN address and supported platform",
        ));
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(error)?
        .as_nanos();
    let name = format!("MightyMonitor-{nonce:x}-{}", address.port());
    // This explicit whitelist is deliberately independent of ShareIdentity/Invitation.
    let properties = [
        ("v", "1"),
        ("secure", "1"),
        ("platform", platform),
        ("mode", if extend { "extend" } else { "mirror" }),
    ];
    ServiceInfo::new(
        SERVICE_TYPE,
        &name,
        &format!("{}.local.", name.to_ascii_lowercase()),
        address.ip(),
        address.port(),
        &properties[..],
    )
    .map_err(error)
}
pub struct Advertisement {
    daemon: ServiceDaemon,
    name: String,
}
impl Advertisement {
    pub fn new(address: SocketAddr, platform: &str, extend: bool) -> io::Result<Self> {
        let info = service(address, platform, extend)?;
        let daemon = ServiceDaemon::new().map_err(error)?;
        // Do not announce the selected private LAN address on unrelated interfaces.
        if let Err(e) = daemon
            .disable_interface(IfKind::All)
            .and_then(|_| daemon.enable_interface(IfKind::Addr(address.ip())))
            .and_then(|_| daemon.register(info.clone()))
        {
            let _ = daemon.shutdown();
            return Err(error(e));
        }
        Ok(Self {
            daemon,
            name: info.get_fullname().to_owned(),
        })
    }
}
impl Drop for Advertisement {
    fn drop(&mut self) {
        if let Ok(done) = self.daemon.unregister(&self.name) {
            let _ = done.recv_timeout(Duration::from_millis(500));
        }
        let _ = self.daemon.shutdown();
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Peer {
    pub name: String,
    pub platform: String,
    pub extend: bool,
    pub addresses: Vec<SocketAddr>,
}
fn peer(info: &ResolvedService) -> Option<Peer> {
    if info.ty_domain != SERVICE_TYPE
        || info.port == 0
        || info.get_property_val_str("v") != Some("1")
        || info.get_property_val_str("secure") != Some("1")
    {
        return None;
    }
    let platform = info.get_property_val_str("platform")?;
    if !["windows", "linux", "android"].contains(&platform) {
        return None;
    }
    let extend = match info.get_property_val_str("mode")? {
        "extend" => true,
        "mirror" => false,
        _ => return None,
    };
    let name = info
        .fullname
        .strip_suffix(&format!(".{SERVICE_TYPE}"))?
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || " -_.".contains(*c))
        .take(64)
        .collect::<String>();
    if name.is_empty() {
        return None;
    }
    let mut addresses = info
        .get_addresses_v4()
        .into_iter()
        .map(IpAddr::V4)
        .filter(|ip| routable(*ip))
        .map(|ip| SocketAddr::new(ip, info.port))
        .collect::<Vec<_>>();
    addresses.sort_unstable();
    addresses.dedup();
    addresses.truncate(8);
    if addresses.is_empty() {
        return None;
    }
    Some(Peer {
        name,
        platform: platform.to_owned(),
        extend,
        addresses,
    })
}
/// Opt-in scan. Bounded UI state; advertised addresses are never connected to.
pub struct Browser {
    daemon: ServiceDaemon,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    peers: Arc<Mutex<BTreeMap<String, Peer>>>,
}
impl Browser {
    pub fn new() -> io::Result<Self> {
        let daemon = ServiceDaemon::new().map_err(error)?;
        let receiver = match daemon.browse(SERVICE_TYPE) {
            Ok(rx) => rx,
            Err(e) => {
                let _ = daemon.shutdown();
                return Err(error(e));
            }
        };
        let stop = Arc::new(AtomicBool::new(false));
        let peers = Arc::new(Mutex::new(BTreeMap::new()));
        let flag = stop.clone();
        let state = peers.clone();
        let worker = match std::thread::Builder::new()
            .name("mm-discovery".into())
            .spawn(move || {
                while !flag.load(Ordering::Acquire) {
                    if let Ok(event) = receiver.recv_timeout(Duration::from_millis(200)) {
                        let mut state = state.lock().unwrap_or_else(|e| e.into_inner());
                        match event {
                            ServiceEvent::ServiceResolved(info) => {
                                if let Some(found) = peer(&info) {
                                    if state.len() < MAX_PEERS || state.contains_key(&info.fullname)
                                    {
                                        state.insert(info.fullname.clone(), found);
                                    }
                                }
                            }
                            ServiceEvent::ServiceRemoved(_, name) => {
                                state.remove(&name);
                            }
                            _ => {}
                        }
                    }
                }
            }) {
            Ok(worker) => worker,
            Err(e) => {
                let _ = daemon.shutdown();
                return Err(e);
            }
        };
        Ok(Self {
            daemon,
            stop,
            worker: Some(worker),
            peers,
        })
    }
    pub fn peers(&self) -> Vec<Peer> {
        self.peers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .cloned()
            .collect()
    }
}
impl Drop for Browser {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = self.daemon.stop_browse(SERVICE_TYPE);
        let _ = self.daemon.shutdown();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn advertisement_contains_only_public_metadata() {
        let info = service("192.168.1.5:5000".parse().unwrap(), "linux", true).unwrap();
        let keys = info
            .get_properties()
            .iter()
            .map(|p| p.key())
            .collect::<Vec<_>>();
        assert_eq!(keys, vec!["v", "secure", "platform", "mode"]);
        let result = peer(&info.as_resolved_service()).unwrap();
        assert!(result.extend);
        assert_eq!(
            result.addresses,
            vec!["192.168.1.5:5000".parse::<SocketAddr>().unwrap()]
        );
    }
    #[test]
    fn invalid_or_legacy_presence_is_not_accepted() {
        for address in [
            "0.0.0.0:5000",
            "127.0.0.1:5000",
            "224.0.0.1:5000",
            "192.168.1.5:0",
        ] {
            assert!(service(address.parse().unwrap(), "linux", false).is_err());
        }
        let info = ServiceInfo::new(
            SERVICE_TYPE,
            "untrusted",
            "untrusted.local.",
            "192.168.1.5",
            5000,
            &[
                ("v", "1"),
                ("secure", "0"),
                ("platform", "linux"),
                ("mode", "mirror"),
            ][..],
        )
        .unwrap();
        assert!(peer(&info.as_resolved_service()).is_none());
    }
}
