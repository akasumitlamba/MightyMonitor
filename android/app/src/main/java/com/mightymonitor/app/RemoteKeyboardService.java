package com.mightymonitor.app;

import android.inputmethodservice.InputMethodService;
import android.view.KeyEvent;
import android.view.inputmethod.InputConnection;

/** Optional user-selected IME for apps/Android versions with limited accessibility editing. */
public final class RemoteKeyboardService extends InputMethodService {
    private static volatile RemoteKeyboardService current;
    @Override public void onCreate() {super.onCreate(); current = this;}
    @Override public void onDestroy() {if (current == this) current = null; super.onDestroy();}
    @Override public android.view.View onCreateInputView() {
        android.widget.Button change = new android.widget.Button(this); change.setText("Switch keyboard"); change.setAllCaps(false); change.setBackgroundColor(android.graphics.Color.WHITE); change.setTextColor(android.graphics.Color.rgb(38,115,77));
        change.setOnClickListener(v -> getSystemService(android.view.inputmethod.InputMethodManager.class).showInputMethodPicker()); return change;
    }
    private static InputConnection connection() {RemoteKeyboardService service = current; return service != null && ScreenShareService.sharing && ScreenShareService.allowControl ? service.getCurrentInputConnection() : null;}
    public static boolean type(String text) {InputConnection connection = connection(); return connection != null && connection.commitText(text, 1);}
    public static boolean key(int hid, boolean pressed, boolean shift, boolean ctrl, boolean alt) {
        InputConnection connection = connection(); if (connection == null) return false;
        int code;
        if (hid >= 4 && hid <= 29) code = KeyEvent.KEYCODE_A + hid - 4;
        else if (hid >= 30 && hid <= 38) code = KeyEvent.KEYCODE_1 + hid - 30;
        else switch (hid) {case 39:code=KeyEvent.KEYCODE_0;break;case 40:code=KeyEvent.KEYCODE_ENTER;break;case 41:return false;case 42:code=KeyEvent.KEYCODE_DEL;break;case 43:code=KeyEvent.KEYCODE_TAB;break;case 44:code=KeyEvent.KEYCODE_SPACE;break;case 79:code=KeyEvent.KEYCODE_DPAD_RIGHT;break;case 80:code=KeyEvent.KEYCODE_DPAD_LEFT;break;case 81:code=KeyEvent.KEYCODE_DPAD_DOWN;break;case 82:code=KeyEvent.KEYCODE_DPAD_UP;break;default:return false;}
        int modifiers = (shift ? KeyEvent.META_SHIFT_ON : 0) | (ctrl ? KeyEvent.META_CTRL_ON : 0) | (alt ? KeyEvent.META_ALT_ON : 0);
        return connection.sendKeyEvent(new KeyEvent(0, android.os.SystemClock.uptimeMillis(), pressed ? KeyEvent.ACTION_DOWN : KeyEvent.ACTION_UP, code, 0, modifiers));
    }
}
