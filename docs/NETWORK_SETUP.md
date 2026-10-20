# Choose the connection network

MightyMonitor uses IP networking. Wi-Fi, Ethernet and an already configured USB
network can carry the same encrypted screen stream; the app does not require a
cloud account or configure network adapters.

On desktop Share, open **Settings → Choose network**, select the interface/address
used by the other device, and start sharing. **Refresh networks** re-enumerates
addresses after plugging in a cable. A manually entered port is preserved when
changing the address. On Android Share, **Network** lists current IPv4 addresses.
The selection is fixed for that sharing session; stop and restart Share after
changing networks, and transfer the new private viewing link.

- **Wi-Fi:** connect both devices to the same LAN. Guest/client-isolated Wi-Fi may
  block both multicast discovery and direct connections. A VPN may use another
  route; choose the address actually reachable by your other device.
- **Ethernet:** select the Ethernet address on both devices. Existing LAN routing
  must allow the sharing port (normally TCP 5000).
- **Android USB tethering:** enable tethering in Android's own settings using a
  data-capable cable, let the PC recognize its network interface, then select its
  address in the app. Driver support and Android/OEM behavior need physical tests.
- **Laptop-to-laptop USB-C:** only supported USB4/Thunderbolt networking hardware,
  cables, drivers and OS configuration create a usable IP interface. A generic
  USB-C cable between ordinary host ports is insufficient. Ethernet is a practical
  alternative. The app does not install networking drivers or weaken security.

Discovery is optional. Use a private link directly when multicast is unavailable.
A loopback address (`127.0.0.1`) works only on the same computer. Address pickers
currently expose IPv4; secure manual IPv6 links remain subject to platform support.
Link-local IPv4 addresses can be useful on direct wired links. Address enumeration
is read-only and never enables tethering, changes routes or opens firewall rules.

The pickers compile on desktop and Android. Physical Wi-Fi/Ethernet/USB4/tethering
acceptance and latency remain unverified. No cable or transport removes the delay
from capture, encoding, transmission and presentation.
