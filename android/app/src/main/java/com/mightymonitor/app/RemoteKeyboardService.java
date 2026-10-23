package com.mightymonitor.app;

import android.inputmethodservice.InputMethodService;
import android.view.KeyEvent;
import android.view.inputmethod.InputConnection;

/** Optional user-selected IME for apps/Android versions with limited accessibility editing. */
public final class RemoteKeyboardService extends InputMethodService {
    private static volatile RemoteKeyboardService current;
    @Override public void onCreate() {super.onCreate(); current = this;}
    private final java.util.LinkedHashMap<Integer, HeldKey> held = new java.util.LinkedHashMap<>();
    private static final class HeldKey {
        final InputConnection connection;
        final long downTime;
        final int modifiers;
        HeldKey(InputConnection connection,long downTime,int modifiers) {this.connection=connection;this.downTime=downTime;this.modifiers=modifiers;}
    }
    private static boolean send(InputConnection connection, KeyEvent event) {
        try {return connection.sendKeyEvent(event);} catch(RuntimeException disconnected) {return false;}
    }
    /** Release only keys sent by this IME, using the connection that received DOWN.
     * Cleanup must work after the sharing/control grant has already been removed. */
    public static void releaseKeys() {
        RemoteKeyboardService service=current;
        if(service==null)return;
        java.util.LinkedHashMap<Integer,HeldKey> releasing=new java.util.LinkedHashMap<>(service.held);
        service.held.clear();
        for(java.util.Map.Entry<Integer,HeldKey> entry:releasing.entrySet()) {
            HeldKey key=entry.getValue();
            send(key.connection,new KeyEvent(key.downTime,android.os.SystemClock.uptimeMillis(),KeyEvent.ACTION_UP,entry.getKey(),0,key.modifiers));
        }
    }
    @Override public void onFinishInput() {releaseKeys();super.onFinishInput();}
    @Override public void onDestroy() {releaseKeys();if (current == this) current = null; super.onDestroy();}
    @Override public boolean onEvaluateFullscreenMode() {return false;}
    @Override public android.view.View onCreateInputView() {
        android.widget.Button change = new android.widget.Button(this); change.setText("Switch keyboard"); change.setAllCaps(false); change.setBackgroundColor(android.graphics.Color.WHITE); change.setTextColor(android.graphics.Color.rgb(38,115,77));
        change.setOnClickListener(v -> getSystemService(android.view.inputmethod.InputMethodManager.class).showInputMethodPicker()); return change;
    }
    private static InputConnection connection() {RemoteKeyboardService service = current; return service != null && ScreenShareService.sharing && ScreenShareService.allowControl ? service.getCurrentInputConnection() : null;}
    public static boolean ready() {return connection() != null;}
    static boolean readyFor(int fieldId) {RemoteKeyboardService service = current; android.view.inputmethod.EditorInfo editor = service != null ? service.getCurrentInputEditorInfo() : null; return ready() && editor != null && editor.fieldId == fieldId;}
    public static boolean type(String text) {InputConnection connection = connection(); return connection != null && connection.commitText(text, 1);}
    public static boolean key(int hid, boolean pressed, boolean shift, boolean ctrl, boolean alt) {
        RemoteKeyboardService service=current; if(service==null)return false;
        int code;
        if (hid >= 4 && hid <= 29) code = KeyEvent.KEYCODE_A + hid - 4;
        else if (hid >= 30 && hid <= 38) code = KeyEvent.KEYCODE_1 + hid - 30;
        else switch (hid) {case 39:code=KeyEvent.KEYCODE_0;break;case 40:code=KeyEvent.KEYCODE_ENTER;break;case 41:return false;case 42:code=KeyEvent.KEYCODE_DEL;break;case 43:code=KeyEvent.KEYCODE_TAB;break;case 44:code=KeyEvent.KEYCODE_SPACE;break;case 79:code=KeyEvent.KEYCODE_DPAD_RIGHT;break;case 80:code=KeyEvent.KEYCODE_DPAD_LEFT;break;case 81:code=KeyEvent.KEYCODE_DPAD_DOWN;break;case 82:code=KeyEvent.KEYCODE_DPAD_UP;break;default:return false;}
        int modifiers = (shift ? KeyEvent.META_SHIFT_ON : 0) | (ctrl ? KeyEvent.META_CTRL_ON : 0) | (alt ? KeyEvent.META_ALT_ON : 0);
        HeldKey existing=service.held.get(code);
        if(!pressed) {
            if(existing==null)return false;
            service.held.remove(code);
            return send(existing.connection,new KeyEvent(existing.downTime,android.os.SystemClock.uptimeMillis(),KeyEvent.ACTION_UP,code,0,modifiers));
        }
        InputConnection connection=connection(); if(connection==null)return false;
        if(existing!=null && existing.connection!=connection) {releaseKeys();existing=null;}
        if(existing==null && service.held.size()>=64) {releaseKeys();return false;}
        long time=android.os.SystemClock.uptimeMillis();
        HeldKey key=existing!=null?existing:new HeldKey(connection,time,modifiers);
        // Retain before delivery so a failed/throwing connection still receives
        // best-effort cleanup; never transfer a held key to another editor.
        service.held.put(code,key);
        return send(connection,new KeyEvent(key.downTime,time,KeyEvent.ACTION_DOWN,code,existing==null?0:1,modifiers));
    }
}
