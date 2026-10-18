package com.mightymonitor.app;

/** Thin JNI boundary: Rust owns framing, typed messages, TCP, feedback and retry. */
public final class NativeBridge {
    static { System.loadLibrary("mightymonitor_android"); }
    private NativeBridge() {}
    public static native long connect(String address);
    public static native byte[] poll(long handle);
    public static native void disconnect(long handle);
}
