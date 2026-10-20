//! Opt-in live mDNS test; advertises public test presence on the selected LAN.
use mightymonitor_discovery::{Advertisement, Browser};
use std::{
    net::{TcpListener, UdpSocket},
    time::{Duration, Instant},
};
#[test]
#[ignore = "Requires multicast-capable LAN; MM_ALLOW_DISCOVERY_SMOKE=1"]
fn advertise_resolve_remove_without_connecting() {
    assert_eq!(
        std::env::var("MM_ALLOW_DISCOVERY_SMOKE").as_deref(),
        Ok("1")
    );
    let route = UdpSocket::bind("0.0.0.0:0").unwrap();
    route.connect("192.0.2.1:9").unwrap();
    let listener = TcpListener::bind((route.local_addr().unwrap().ip(), 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let browser = Browser::new().unwrap();
    let publisher = Advertisement::new(address, "linux", false).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    while !browser
        .peers()
        .iter()
        .any(|peer| peer.addresses.contains(&address))
    {
        assert!(Instant::now() < deadline, "mDNS presence was not resolved");
        std::thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock,
        "Presence must not initiate a screen connection"
    );
    drop(publisher);
    let deadline = Instant::now() + Duration::from_secs(10);
    while browser
        .peers()
        .iter()
        .any(|peer| peer.addresses.contains(&address))
    {
        assert!(
            Instant::now() < deadline,
            "Stopped sharing remained discoverable"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}
