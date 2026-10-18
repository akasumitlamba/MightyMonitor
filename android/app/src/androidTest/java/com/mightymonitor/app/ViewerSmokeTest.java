package com.mightymonitor.app;
import android.widget.Button;
import android.widget.CheckBox;
import android.widget.EditText;
import androidx.test.ext.junit.runners.AndroidJUnit4;
import androidx.test.rule.ActivityTestRule;
import androidx.test.platform.app.InstrumentationRegistry;
import org.junit.Rule;
import org.junit.Test;
import org.junit.runner.RunWith;
import static org.junit.Assert.assertTrue;
@RunWith(AndroidJUnit4.class)
public final class ViewerSmokeTest {
    @Rule public ActivityTestRule<MainActivity> activity = new ActivityTestRule<>(MainActivity.class);
    @Test public void decodesRealDesktopH264ThroughRustProtocol() throws Exception {
        MainActivity app = activity.getActivity();
        long deadline = System.currentTimeMillis() + 60000;
        while (!((Button) app.findViewById(MainActivity.CONNECT_ID)).isEnabled() && System.currentTimeMillis() < deadline) Thread.sleep(100);
        InstrumentationRegistry.getInstrumentation().runOnMainSync(() -> {
            ((EditText) app.findViewById(MainActivity.ADDRESS_ID)).setText("10.0.2.2:5000");
            ((CheckBox) app.findViewById(MainActivity.TRUST_ID)).setChecked(true);
            app.findViewById(MainActivity.CONNECT_ID).performClick();
        });
        while (app.getRenderedFrames() < 3 && System.currentTimeMillis() < deadline) Thread.sleep(100);
        assertTrue("Android MediaCodec never presented desktop H.264 frames", app.getRenderedFrames() >= 3);
    }

    @Test public void capturesPhoneScreenAndDecodesThroughRustHost() throws Exception {
        MainActivity app = activity.getActivity();
        InstrumentationRegistry.getInstrumentation().runOnMainSync(() -> {
            app.findViewById(MainActivity.SHARE_TAB_ID).performClick();
            ((CheckBox) app.findViewById(MainActivity.SHARE_TRUST_ID)).setChecked(true);
            app.findViewById(MainActivity.SHARE_ID).performClick();
        });
        androidx.test.uiautomator.UiDevice device = androidx.test.uiautomator.UiDevice.getInstance(InstrumentationRegistry.getInstrumentation());
        androidx.test.uiautomator.UiObject2 confirm = device.wait(androidx.test.uiautomator.Until.findObject(androidx.test.uiautomator.By.res("android", "button1")), 20000);
        assertTrue("Android screen capture consent did not appear", confirm != null); confirm.click();
        long client = 0; android.media.MediaCodec decoder = null;
        try {
            long deadline = System.currentTimeMillis() + 60000;
            while (ScreenShareService.capturedFrames < 3 && System.currentTimeMillis() < deadline) Thread.sleep(100);
            assertTrue("MediaProjection produced no encoded screen frames: " + ScreenShareService.summary, ScreenShareService.capturedFrames >= 3);
            client = NativeBridge.connect("127.0.0.1:5000"); int decoded = 0; byte[] pending = null;
            android.media.MediaCodec.BufferInfo info = new android.media.MediaCodec.BufferInfo();
            while (decoded < 3 && System.currentTimeMillis() < deadline) {
                if (decoder != null) { int out = decoder.dequeueOutputBuffer(info, 0); if (out >= 0) { decoder.releaseOutputBuffer(out, false); decoded++; } }
                byte[] event = pending != null ? pending : NativeBridge.poll(client);
                if (event == null) { Thread.sleep(10); continue; }
                if (event[0] == 2) {
                    java.nio.ByteBuffer config = java.nio.ByteBuffer.wrap(event).order(java.nio.ByteOrder.LITTLE_ENDIAN);
                    decoder = android.media.MediaCodec.createDecoderByType("video/avc");
                    android.media.MediaFormat format = android.media.MediaFormat.createVideoFormat("video/avc", config.getInt(1), config.getInt(5));
                    format.setInteger(android.media.MediaFormat.KEY_MAX_INPUT_SIZE, 16 * 1024 * 1024);
                    decoder.configure(format, null, null, 0); decoder.start();
                } else if (event[0] == 3 && decoder != null) {
                    int input = decoder.dequeueInputBuffer(0);
                    if (input < 0) { pending = event; Thread.sleep(10); continue; }
                    java.nio.ByteBuffer buffer = decoder.getInputBuffer(input); buffer.clear(); buffer.put(event, 10, event.length - 10);
                    decoder.queueInputBuffer(input, 0, event.length - 10, java.nio.ByteBuffer.wrap(event).order(java.nio.ByteOrder.LITTLE_ENDIAN).getLong(1), event[9] != 0 ? android.media.MediaCodec.BUFFER_FLAG_KEY_FRAME : 0);
                    pending = null;
                }
            }
            assertTrue("Android capture -> Rust host -> Rust viewer -> decoder produced no video", decoded >= 3);
        } finally {
            if (client != 0) NativeBridge.disconnect(client);
            if (decoder != null) { decoder.stop(); decoder.release(); }
            app.stopService(new android.content.Intent(app, ScreenShareService.class));
        }
    }
}
