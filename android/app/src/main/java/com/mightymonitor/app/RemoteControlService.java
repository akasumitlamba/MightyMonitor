package com.mightymonitor.app;

import android.accessibilityservice.AccessibilityService;
import android.accessibilityservice.GestureDescription;
import android.graphics.Path;
import android.graphics.PointF;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.view.accessibility.AccessibilityEvent;
import android.view.accessibility.AccessibilityNodeInfo;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;

/** Optional system-authorized input. No input runs outside an explicitly shared session. */
public final class RemoteControlService extends AccessibilityService {
    private static volatile RemoteControlService current;
    private final Handler main = new Handler(Looper.getMainLooper());
    private float x, y, strokeX, strokeY;
    private boolean down, busy, changed, shift, ctrl, alt, pointerValid, pendingPress;
    private final Runnable beginPress = this::drainGesture;
    private GestureDescription.StrokeDescription stroke;
    private static final int MAX_PENDING_INPUT=128;
    private static final class PendingInput {
        final ScreenShareService owner; final byte[] event;
        PendingInput(ScreenShareService owner,byte[] event){this.owner=owner;this.event=event;}
    }
    private final java.util.ArrayDeque<PendingInput> pendingInputs=new java.util.ArrayDeque<>();
    private boolean inputDrainScheduled;
    private ScreenShareService overflowOwner;
    private long received, releases, dispatched, completed, cancelled, rejected, invalidPoints, blocked, failed;
    // Bounded counters only: no coordinates, text, keys or invitation data.
    // Called on the main thread by instrumentation, never broadcast or logged by the app.
    static String diagnosticState() {
        RemoteControlService service = current;
        if (service == null) return "service=disconnected";
        return "received="+service.received+", releases="+service.releases+", dispatched="+service.dispatched
            +", completed="+service.completed+", cancelled="+service.cancelled+", rejected="+service.rejected
            +", invalidPoints="+service.invalidPoints+", blocked="+service.blocked+", failed="+service.failed+", down="+service.down+", busy="+service.busy
            +", stroke="+(service.stroke!=null)+", allowed="+allowed();
    }
    public static boolean available() { return current != null; }
    private static boolean allowed() { return ScreenShareService.sharing && ScreenShareService.allowControl; }
    @Override protected void onServiceConnected() { current = this; main.post(watch); }
    @Override public void onAccessibilityEvent(AccessibilityEvent event) { }
    @Override public void onInterrupt() { release(); }
    @Override public void onDestroy() { if (current == this) current = null; release(); synchronized(pendingInputs){pendingInputs.clear();overflowOwner=null;inputDrainScheduled=false;} main.removeCallbacksAndMessages(null); super.onDestroy(); }
    public static void stopInput() {
        RemoteControlService service=current;
        if(service!=null) {
            synchronized(service.pendingInputs){service.pendingInputs.clear();}
            service.main.post(service::release);
        }
    }
    public static void accept(byte[] event) {accept(event,ScreenShareService.inputOwner());}
    static void accept(byte[] event,ScreenShareService owner) {
        RemoteControlService service=current;
        if(service==null || !ScreenShareService.allowsInput(owner) || event==null || event.length==0 || event.length>4097)return;
        synchronized(service.pendingInputs) {
            if(service.overflowOwner!=null)return;
            if(service.pendingInputs.size()>=MAX_PENDING_INPUT) {
                service.pendingInputs.clear();service.overflowOwner=owner;
            } else service.pendingInputs.addLast(new PendingInput(owner,event.clone()));
            if(!service.inputDrainScheduled) {
                service.inputDrainScheduled=true;service.main.post(service::drainInputs);
            }
        }
    }
    private void drainInputs() {
        // One scheduled drain and a bounded queue: a slow UI must not accumulate
        // unbounded Handler callbacks or replay an old share into a later share.
        for(int i=0;i<32;i++) {
            PendingInput input;ScreenShareService overflow;
            synchronized(pendingInputs) {
                overflow=overflowOwner;overflowOwner=null;
                if(overflow!=null){pendingInputs.clear();inputDrainScheduled=false;input=null;}
                else {input=pendingInputs.pollFirst();if(input==null)inputDrainScheduled=false;}
            }
            if(overflow!=null) {
                if(ScreenShareService.allowsInput(overflow))ScreenShareService.revokeControl();
                release();return;
            }
            if(input==null)return;
            if(!ScreenShareService.allowsInput(input.owner)){blocked++;release();continue;}
            try {apply(input.event);} catch(RuntimeException ignored){failed++;release();}
        }
        main.post(this::drainInputs);
    }
    private final Runnable watch = new Runnable() {
        @Override public void run() { if (!allowed()) release(); if (current == RemoteControlService.this) main.postDelayed(this, 100); }
    };
    private void release() { RemoteKeyboardService.releaseKeys(); releases++; pendingPress=false; main.removeCallbacks(beginPress); pointerValid = false; down = false; changed = true; shift = ctrl = alt = false; drainGesture(); }
    private PointF map(float nx, float ny) {
        if (!Float.isFinite(nx) || !Float.isFinite(ny) || nx < 0 || nx > 1 || ny < 0 || ny > 1) return null;
        android.util.DisplayMetrics metrics = new android.util.DisplayMetrics();
        getSystemService(android.view.WindowManager.class).getDefaultDisplay().getRealMetrics(metrics);
        float cw = ScreenShareService.captureWidth, ch = ScreenShareService.captureHeight;
        if (cw <= 0 || ch <= 0) return null;
        float scale = Math.min(cw / metrics.widthPixels, ch / metrics.heightPixels);
        float px = (nx * cw - (cw - metrics.widthPixels * scale) / 2) / scale;
        float py = (ny * ch - (ch - metrics.heightPixels * scale) / 2) / scale;
        if (!down && (px < 0 || py < 0 || px >= metrics.widthPixels || py >= metrics.heightPixels)) return null;
        return new PointF(Math.max(0, Math.min(metrics.widthPixels - 1, px)), Math.max(0, Math.min(metrics.heightPixels - 1, py)));
    }
    private void point(float nx, float ny) { PointF p = map(nx, ny); pointerValid = p != null; if(p==null)invalidPoints++; if (p != null) {x = p.x; y = p.y; changed = true; drainGesture();} }
    private void press() {
        down = true; changed = true;
        if (!busy && stroke == null) {
            // Coalesce a quick down/up into one complete tap instead of a
            // continued stroke whose final segment can be cancelled by Android.
            pendingPress = true;
            main.removeCallbacks(beginPress);
            main.postDelayed(beginPress, 32);
        } else drainGesture();
    }
    private void drainGesture() {
        if (busy || (!changed && !down)) return;
        if (stroke == null && !down && !pendingPress) {changed = false; return;}
        main.removeCallbacks(beginPress);
        pendingPress = false;
        if (down && !allowed()) down = false;
        Path path = new Path();
        if (stroke == null) path.moveTo(x, y); else {path.moveTo(strokeX, strokeY); path.lineTo(x, y);}
        final boolean continuing = down;
        // Complete taps need enough duration for Android's input sampling. Keep
        // drag segments short, but do not synthesize a sub-frame tap.
        long duration = stroke == null && !continuing ? android.view.ViewConfiguration.getTapTimeout() : 16;
        GestureDescription.StrokeDescription next = stroke == null ? new GestureDescription.StrokeDescription(path, 0, duration, continuing) : stroke.continueStroke(path, 0, duration, continuing);
        strokeX = x; strokeY = y; changed = false; busy = true;
        boolean accepted = dispatchGesture(new GestureDescription.Builder().addStroke(next).build(), new GestureResultCallback() {
            @Override public void onCompleted(GestureDescription description) {completed++; busy = false; stroke = continuing ? next : null; if (changed || !down) drainGesture();}
            @Override public void onCancelled(GestureDescription description) {cancelled++; busy = false; stroke = null; down = false; changed = false;}
        }, main);
        if (accepted) dispatched++;
        if (!accepted) {rejected++; busy = false; stroke = null; down = false; changed = false;}
    }
    private void apply(byte[] event) {
        if (event.length == 0) return;
        received++;
        ByteBuffer bytes = ByteBuffer.wrap(event).order(ByteOrder.LITTLE_ENDIAN);
        switch (event[0]) {
            case 1: if (event.length == 9) point(bytes.getFloat(1), bytes.getFloat(5)); break;
            case 2:
                if (event.length != 3 || (event[2] != 0 && !pointerValid)) break;
                if (event[1] == 0) {
                    if (event[2] != 0) press();
                    else { down = false; changed = true; drainGesture(); }
                }
                else if (event[1] == 1 && event[2] != 0) performGlobalAction(GLOBAL_ACTION_BACK);
                else if (event[1] == 2 && event[2] != 0) performGlobalAction(GLOBAL_ACTION_HOME);
                break;
            case 3:
                if (event.length != 10) break;
                if (event[1] == 0) {PointF p = map(bytes.getFloat(2), bytes.getFloat(6)); if (p == null) {invalidPoints++;pointerValid=false;break;} pointerValid=true; x = p.x; y = p.y; press();}
                else if (event[1] == 1) point(bytes.getFloat(2), bytes.getFloat(6));
                else {point(bytes.getFloat(2), bytes.getFloat(6)); down = false; changed = true; drainGesture();}
                break;
            case 4: if (event.length == 6) key(bytes.getInt(1), event[5] != 0); break;
            case 5: if (event.length > 1 && event.length <= 4097) {release(); type(new String(event, 1, event.length - 1, StandardCharsets.UTF_8));} break;
            case 6: if (event.length == 9) scroll(bytes.getFloat(1), bytes.getFloat(5)); break;
            case 7: release(); break;
            default: break;
        }
    }
    private void scroll(float dx, float dy) {
        if (busy || down || !Float.isFinite(dx) || !Float.isFinite(dy)) return;
        android.util.DisplayMetrics metrics = new android.util.DisplayMetrics(); getSystemService(android.view.WindowManager.class).getDefaultDisplay().getRealMetrics(metrics);
        float cx = metrics.widthPixels / 2f, cy = metrics.heightPixels / 2f;
        Path path = new Path(); path.moveTo(cx, cy);
        path.lineTo(Math.max(1, Math.min(metrics.widthPixels - 1, cx - dx * 80)), Math.max(1, Math.min(metrics.heightPixels - 1, cy - dy * 80)));
        dispatchGesture(new GestureDescription.Builder().addStroke(new GestureDescription.StrokeDescription(path, 0, 180)).build(), null, main);
    }
    private AccessibilityNodeInfo editor() {
        AccessibilityNodeInfo root = getRootInActiveWindow(); if (root == null) return null;
        AccessibilityNodeInfo focused = root.findFocus(AccessibilityNodeInfo.FOCUS_INPUT);
        return focused != null && focused.isEditable() ? focused : null;
    }
    private void type(String text) {
        if (!allowed()) return;
        if (RemoteKeyboardService.type(text)) return;
        AccessibilityNodeInfo node = editor(); if (node == null) return;
        String old = node.getText() == null ? "" : node.getText().toString();
        if (old.length() > 65536) return;
        int start = node.getTextSelectionStart(), end = node.getTextSelectionEnd();
        if (start < 0 || end < 0 || start > old.length() || end > old.length()) start = end = old.length();
        if (start > end) {int swap = start; start = end; end = swap;}
        Bundle value = new Bundle(); value.putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, node.isPassword() ? text : old.substring(0, start) + text + old.substring(end));
        node.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, value);
    }
    private void key(int code, boolean pressed) {
        if (code == 224 || code == 228) ctrl = pressed;
        if (code == 225 || code == 229) shift = pressed;
        if (code == 226 || code == 230) alt = pressed;
        if (RemoteKeyboardService.key(code, pressed, shift, ctrl, alt)) return;
        if (!pressed) return;
        if (code == 41) {performGlobalAction(GLOBAL_ACTION_BACK); return;}
        AccessibilityNodeInfo node = editor();
        if (code == 40) {if (node != null && android.os.Build.VERSION.SDK_INT >= 30) node.performAction(AccessibilityNodeInfo.AccessibilityAction.ACTION_IME_ENTER.getId()); return;}
        if (ctrl && node != null) {
            if (code == 4) {Bundle selection = new Bundle(); selection.putInt(AccessibilityNodeInfo.ACTION_ARGUMENT_SELECTION_START_INT, 0); selection.putInt(AccessibilityNodeInfo.ACTION_ARGUMENT_SELECTION_END_INT, node.getText() == null ? 0 : node.getText().length()); node.performAction(AccessibilityNodeInfo.ACTION_SET_SELECTION, selection);}
            else if (code == 6) node.performAction(AccessibilityNodeInfo.ACTION_COPY);
            else if (code == 25) node.performAction(AccessibilityNodeInfo.ACTION_PASTE);
            else if (code == 27) node.performAction(AccessibilityNodeInfo.ACTION_CUT);
            return;
        }
        if (code == 42 && node != null) {
            String text = node.getText() == null ? "" : node.getText().toString(); int start = node.getTextSelectionStart(), end = node.getTextSelectionEnd();
            if (start < 0 || end < 0 || start > text.length() || end > text.length()) start = end = text.length();
            if (start == end && start > 0) start = text.offsetByCodePoints(start, -1);
            Bundle value = new Bundle(); value.putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, text.substring(0, Math.min(start, end)) + text.substring(Math.max(start, end))); node.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, value); return;
        }
        if (alt || ctrl) return;
        if (code >= 4 && code <= 29) type(String.valueOf((char)((shift ? 'A' : 'a') + code - 4)));
        else if (code >= 30 && code <= 38) type(String.valueOf((shift ? "!@#$%^&*(" : "123456789").charAt(code - 30)));
        else if (code == 39) type(shift ? ")" : "0");
        else if (code == 44) type(" ");
        else {String plain = "-=[]\\;\'`,./", upper = "_+{}|:\"~<>?"; int[] codes = {45,46,47,48,49,51,52,53,54,55,56}; for (int i = 0; i < codes.length; i++) if (code == codes[i]) type(String.valueOf((shift ? upper : plain).charAt(i)));}
    }
}
