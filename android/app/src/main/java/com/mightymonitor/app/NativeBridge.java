package com.mightymonitor.app;

/** Thin JNI boundary: Rust owns framing, typed messages, TCP, feedback and retry. */
public final class NativeBridge {
    static { System.loadLibrary("mightymonitor_android"); }
    private NativeBridge() {}
    public static native long connect(String address);
    public static native byte[] poll(long handle);
    public static native boolean requestControl(long handle, boolean enabled);
    public static native boolean pointer(long handle, float x, float y, int phase);
    public static native boolean key(long handle, int code, boolean pressed);
    public static native boolean typeText(long handle, String text);
    public static native void disconnect(long handle);
    public static native long startHost(String address, int width, int height, int fps);
    public static native boolean publish(long handle, byte[] data, long timestampUs, boolean keyframe);
    public static native boolean hostNeedsKeyframe(long handle);
    public static native int hostBitrate(long handle);
    public static native String hostInvite(long handle, String address);
    public static native String hostStatus(long handle);
    public static native void stopHost(long handle);
}
