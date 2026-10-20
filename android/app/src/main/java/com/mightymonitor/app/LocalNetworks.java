package com.mightymonitor.app;
import java.net.*;
import java.util.*;
/** Read-only interface selection. Does not enable tethering or change routing. */
final class LocalNetworks {
    static LinkedHashMap<String,String> addresses() {
        LinkedHashMap<String,String> result=new LinkedHashMap<>();
        try {for(NetworkInterface network:Collections.list(NetworkInterface.getNetworkInterfaces())){
            if(!network.isUp() || network.isLoopback())continue;
            for(InetAddress address:Collections.list(network.getInetAddresses())){
                if(!(address instanceof Inet4Address) || address.isAnyLocalAddress() || address.isLoopbackAddress() || address.isMulticastAddress())continue;
                String ip=address.getHostAddress();if(ip==null || ip.equals("255.255.255.255"))continue;
                if(result.size()<64)result.put(network.getName()+" · "+ip,ip+":5000");
            }
        }} catch(SocketException ignored){}
        return result;
    }
    static String defaultAddress(){Collection<String> addresses=addresses().values();return addresses.isEmpty()?"Connect to a local network":addresses.iterator().next();}
    static String validate(String selected){if(selected==null)return defaultAddress();if(addresses().containsValue(selected))return selected;throw new IllegalArgumentException("Selected network is unavailable. Choose Network and try again.");}
}
