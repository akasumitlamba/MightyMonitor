package com.mightymonitor.app;

import android.content.Context;
import android.net.nsd.NsdManager;
import android.net.nsd.NsdServiceInfo;
import android.os.Handler;
import android.os.Looper;
import java.nio.charset.StandardCharsets;
import java.util.*;

/** Untrusted presence only. Never publishes or consumes private viewing credentials. */
final class NearbyDevices {
    static final String TYPE = "_mightymonitor._tcp.";
    private final NsdManager manager;
    private final Handler main = new Handler(Looper.getMainLooper());
    private final java.util.function.Consumer<String> update;
    private final LinkedHashMap<String,String> peers = new LinkedHashMap<>();
    private final Set<String> present = new HashSet<>();
    private final ArrayDeque<NsdServiceInfo> pending = new ArrayDeque<>();
    private NsdManager.DiscoveryListener listener;
    private int generation;
    private boolean resolving;
    NearbyDevices(Context context, java.util.function.Consumer<String> update) {manager=context.getSystemService(NsdManager.class);this.update=update;}
    void start() {
        stop(); final int run=++generation;
        listener = new NsdManager.DiscoveryListener() {
            public void onDiscoveryStarted(String type) {}
            public void onDiscoveryStopped(String type) {}
            public void onStartDiscoveryFailed(String type,int code) {main.post(()->{if(run==generation){stop();update.accept("Discovery unavailable. You can still paste a private link.");}});}
            public void onStopDiscoveryFailed(String type,int code) {}
            public void onServiceFound(NsdServiceInfo info) {main.post(()->{if(run!=generation || present.size()>=32 || pending.size()>=32 || !(TYPE.equals(info.getServiceType()) || (TYPE+"local.").equals(info.getServiceType())))return;if(present.add(info.getServiceName())){pending.add(info);resolve(run);}});}
            public void onServiceLost(NsdServiceInfo info) {main.post(()->{if(run!=generation)return;present.remove(info.getServiceName());peers.remove(info.getServiceName());render();});}
        };
        try {manager.discoverServices(TYPE,NsdManager.PROTOCOL_DNS_SD,listener);render();} catch(RuntimeException error) {stop();update.accept("Discovery unavailable. You can still paste a private link.");}
    }
    private void resolve(int run) {
        if(resolving || pending.isEmpty() || run!=generation)return;
        NsdServiceInfo next=pending.remove();resolving=true;
        NsdManager.ResolveListener callback=new NsdManager.ResolveListener(){
            public void onResolveFailed(NsdServiceInfo info,int code){main.post(()->{if(run!=generation)return;resolving=false;resolve(run);});}
            public void onServiceResolved(NsdServiceInfo info){main.post(()->{if(run!=generation)return;resolving=false;String value=describe(info);if(value!=null && present.contains(info.getServiceName()))peers.put(info.getServiceName(),value);render();resolve(run);});}
        };
        try {manager.resolveService(next,callback);} catch(RuntimeException error){resolving=false;resolve(run);}
    }
    static String describe(NsdServiceInfo info) {
        Map<String,byte[]> txt=info.getAttributes();
        if(!"1".equals(value(txt,"v")) || !"1".equals(value(txt,"secure")))return null;
        String platform=value(txt,"platform"),mode=value(txt,"mode");
        if(!Arrays.asList("windows","linux","android").contains(platform) || !("mirror".equals(mode)||"extend".equals(mode)))return null;
        java.net.InetAddress host=info.getHost();
        if(host==null || host.isLoopbackAddress() || host.isAnyLocalAddress() || host.isMulticastAddress() || info.getPort()<1 || info.getPort()>65535)return null;
        String address=host.getHostAddress(); if(address==null)return null;
        return platform+" · "+("extend".equals(mode)?"Extra display":"Screen")+" · "+(address.contains(":")?"["+address+"]":address)+":"+info.getPort();
    }
    private static String value(Map<String,byte[]> txt,String key){byte[] value=txt.get(key);return value==null || value.length>32 ? "" : new String(value,StandardCharsets.UTF_8);}
    private void render(){update.accept(peers.isEmpty()?"Looking for devices. Enable Show on local network on the sharing device.":String.join("\n",peers.values())+"\nPaste the private link from the sharing device to connect.");}
    void stop(){++generation;NsdManager.DiscoveryListener old=listener;listener=null;if(old!=null)try{manager.stopServiceDiscovery(old);}catch(RuntimeException ignored){}peers.clear();present.clear();pending.clear();resolving=false;}
    static final class Advertisement {
        private final NsdManager manager;
        private volatile boolean closed,registered;
        private final NsdManager.RegistrationListener listener = new NsdManager.RegistrationListener(){
            public void onRegistrationFailed(NsdServiceInfo info,int code) {}
            public void onUnregistrationFailed(NsdServiceInfo info,int code) {}
            public void onServiceRegistered(NsdServiceInfo info){registered=true;if(closed)close();}
            public void onServiceUnregistered(NsdServiceInfo info){registered=false;}
        };
        Advertisement(Context context,int port){manager=context.getSystemService(NsdManager.class);NsdServiceInfo info=new NsdServiceInfo();info.setServiceType(TYPE);info.setServiceName("MightyMonitor Android");info.setPort(port);info.setAttribute("v","1");info.setAttribute("secure","1");info.setAttribute("platform","android");info.setAttribute("mode","mirror");try{manager.registerService(info,NsdManager.PROTOCOL_DNS_SD,listener);}catch(RuntimeException ignored){closed=true;}}
        void close(){closed=true;if(registered){registered=false;try{manager.unregisterService(listener);}catch(RuntimeException ignored){}}}
    }
}
