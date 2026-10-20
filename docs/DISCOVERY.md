# Nearby devices (development)

Discovery finds sharing addresses on a local multicast-capable network. It does
not pair devices, transfer a private viewing link or grant permission to view or
control a screen. You still obtain the private link directly from the sharing
device and explicitly connect. Persistent pairing is separate unfinished work.

On a desktop, expand **Nearby devices** under View and enable **Find nearby
sharing devices**. On Android, enable the same checkbox in View. Before starting
Share, optionally enable **Show on local network** on the source device. Both
options default off. Stop sharing removes its advertisement; stop scanning to
stop browsing. Android stops its scan when its Activity goes into the background.
The CLI equivalent is `mm-host --secure --discoverable --bind YOUR_LAN_IP:5000`.

The service type is `_mightymonitor._tcp.local.`. TXT data has only four fields:
`v=1`, `secure=1`, `platform=windows|linux|android`, and `mode=mirror|extend`.
Desktop service names are anonymous session-local labels, not the machine's
hostname. Android uses a generic service name. No certificate fingerprint,
bearer token, viewing link, keyboard permission or stored identity is advertised.
Any network peer can spoof this public metadata: it is never a trust signal.

Desktop advertisements use the explicitly selected sharing interface. The first
browser implementation lists IPv4 addresses. Android uses the platform NSD
service and may resolve IPv4 or IPv6. Client lists hold at most 32 devices and
validate the metadata before displaying it. Discovery does not open TCP screen
connections, enable legacy mode, or change a viewer's connection field.

mDNS may be blocked by guest Wi-Fi, client isolation, VPNs, firewalls, multicast
filtering or different subnets. Private links remain usable when scanning is
unavailable. This milestone does not configure firewalls or network adapters.

## Evidence

The Rust discovery unit tests check the metadata whitelist and rejection of
invalid/plaintext presence. An opt-in real local multicast test passed advertise,
resolve, removal on Stop, and absence of a TCP connection attempt. Run only when
local test advertising is appropriate:

```sh
MM_ALLOW_DISCOVERY_SMOKE=1 cargo test -p mightymonitor-discovery --test mdns_presence -- --ignored
```

Android source/resources compile locally. Android NSD discovery/removal acceptance
has been added to the emulator workflow alongside existing video/control tests;
results are not yet established. Physical Windows/Linux/Android discovery,
network changes and multicast-restricted networks still need device testing.
