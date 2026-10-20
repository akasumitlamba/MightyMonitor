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
    @org.junit.Before public void preserveAccessibilityService() {
        androidx.test.uiautomator.Configurator.getInstance().setUiAutomationFlags(android.app.UiAutomation.FLAG_DONT_SUPPRESS_ACCESSIBILITY_SERVICES);
        android.app.UiAutomation automation = InstrumentationRegistry.getInstrumentation().getUiAutomation(android.app.UiAutomation.FLAG_DONT_SUPPRESS_ACCESSIBILITY_SERVICES);
        if ("true".equals(InstrumentationRegistry.getArguments().getString("remoteControlTest"))) {
            // Instrumentation restarts the target app. Rebind its test service afterwards;
            // pre-launch grants can be marked crashed/suppressed by that process restart.
            shell(automation, "settings put secure enabled_accessibility_services null");
            // The enabled-service observer must see a real removal to clear the
            // crashed-service entry left by am instrument restarting this process.
            android.os.SystemClock.sleep(1500);
            shell(automation, "settings put secure enabled_accessibility_services com.mightymonitor.app/.RemoteControlService");
            shell(automation, "settings put secure accessibility_enabled 1");
        }
    }
    private static void shell(android.app.UiAutomation automation, String command) {
        try (java.io.InputStream output = new android.os.ParcelFileDescriptor.AutoCloseInputStream(automation.executeShellCommand(command))) {byte[] buffer = new byte[512]; while (output.read(buffer) >= 0) {}}
        catch (java.io.IOException error) {throw new IllegalStateException("Test system grant failed", error);}
    }
    @Rule public ActivityTestRule<MainActivity> activity = new ActivityTestRule<>(MainActivity.class);
    @Test public void discoversAndRemovesLocalPublicPresence() throws Exception {
        MainActivity app=activity.getActivity();
        java.net.ServerSocket listener=new java.net.ServerSocket(0);
        java.util.concurrent.atomic.AtomicReference<String> status=new java.util.concurrent.atomic.AtomicReference<>("");
        NearbyDevices browser=new NearbyDevices(app,status::set);
        NearbyDevices.Advertisement publisher=new NearbyDevices.Advertisement(app,listener.getLocalPort());
        InstrumentationRegistry.getInstrumentation().runOnMainSync(browser::start);
        try {
            long deadline=System.currentTimeMillis()+30000;
            String port=":"+listener.getLocalPort();
            while(!status.get().contains(port) && System.currentTimeMillis()<deadline)Thread.sleep(100);
            assertTrue("Android NSD did not resolve the test sharing presence",status.get().contains(port));
            assertTrue("Presence leaked a viewing credential",!status.get().contains("mightymonitor://"));
            listener.setSoTimeout(200);boolean connected=false;try{listener.accept().close();connected=true;}catch(java.net.SocketTimeoutException expected){}
            assertTrue("Discovery initiated a screen connection",!connected);
            publisher.close();deadline=System.currentTimeMillis()+15000;
            while(status.get().contains(port) && System.currentTimeMillis()<deadline)Thread.sleep(100);
            assertTrue("Stopped sharing remained in Android discovery",!status.get().contains(port));
        } finally {publisher.close();listener.close();InstrumentationRegistry.getInstrumentation().runOnMainSync(browser::stop);}
    }
    @Test public void decodesRealDesktopH264ThroughRustProtocol() throws Exception {
        MainActivity app = activity.getActivity();
        long deadline = System.currentTimeMillis() + 60000;
        while (!((Button) app.findViewById(MainActivity.CONNECT_ID)).isEnabled() && System.currentTimeMillis() < deadline) Thread.sleep(100);
        saveScreenshot("view");
        final String pcInvitation = new String(java.nio.file.Files.readAllBytes(new java.io.File(app.getFilesDir(), "pc-invitation").toPath()), java.nio.charset.StandardCharsets.UTF_8).trim();
        InstrumentationRegistry.getInstrumentation().runOnMainSync(() -> {
            ((EditText) app.findViewById(MainActivity.ADDRESS_ID)).setText(pcInvitation);
            app.findViewById(MainActivity.CONNECT_ID).performClick();
        });
        while (app.getRenderedFrames() < 3 && System.currentTimeMillis() < deadline) Thread.sleep(100);
        assertTrue("Android MediaCodec never presented desktop H.264 frames", app.getRenderedFrames() >= 3);
        int before = app.getRenderedFrames();
        InstrumentationRegistry.getInstrumentation().runOnMainSync(() -> app.findViewById(MainActivity.FULLSCREEN_ID).performClick());
        deadline = System.currentTimeMillis() + 15000;
        while (app.getRenderedFrames() < before + 3 && System.currentTimeMillis() < deadline) Thread.sleep(100);
        assertTrue("Video stopped entering full screen", app.getRenderedFrames() >= before + 3);
        assertTrue("Full screen exit is not available", app.findViewById(MainActivity.EXIT_FULLSCREEN_ID).getVisibility() == android.view.View.VISIBLE);
        before = app.getRenderedFrames();
        InstrumentationRegistry.getInstrumentation().runOnMainSync(() -> app.findViewById(MainActivity.EXIT_FULLSCREEN_ID).performClick());
        deadline = System.currentTimeMillis() + 15000;
        while (app.getRenderedFrames() < before + 3 && System.currentTimeMillis() < deadline) Thread.sleep(100);
        assertTrue("Video stopped leaving full screen", app.getRenderedFrames() >= before + 3);
    }

    @Test public void capturesPhoneScreenAndDecodesThroughRustHost() throws Exception {
        MainActivity app = activity.getActivity();
        java.io.File desktopMarker = new java.io.File(app.getFilesDir(), "desktop-video-ok");
        desktopMarker.delete();
        InstrumentationRegistry.getInstrumentation().runOnMainSync(() -> {
            app.findViewById(MainActivity.SHARE_TAB_ID).performClick();
            app.findViewById(MainActivity.SHARE_ID).performClick();
        });
        androidx.test.uiautomator.UiDevice device = androidx.test.uiautomator.UiDevice.getInstance(InstrumentationRegistry.getInstrumentation());
        androidx.test.uiautomator.UiObject2 confirm = captureConsentAfterOptionalNotificationDenial(device);
        assertTrue("Android screen capture consent did not appear", confirm != null); confirm.click();
        long client = 0; android.media.MediaCodec decoder = null;
        try {
            long deadline = System.currentTimeMillis() + 60000;
            while (ScreenShareService.capturedFrames < 3 && System.currentTimeMillis() < deadline) Thread.sleep(100);
            assertTrue("MediaProjection produced no encoded screen frames: " + ScreenShareService.summary, ScreenShareService.capturedFrames >= 3);
            saveScreenshot("share");
            client = NativeBridge.connect(ScreenShareService.invitation.replaceFirst("mightymonitor://[^/]+/", "mightymonitor://127.0.0.1:5000/")); int decoded = 0; byte[] pending = null;
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
            NativeBridge.disconnect(client); client = 0;
            java.nio.file.Files.write(new java.io.File(app.getFilesDir(), "share-invitation").toPath(), ScreenShareService.invitation.getBytes(java.nio.charset.StandardCharsets.UTF_8));
            if ("true".equals(InstrumentationRegistry.getArguments().getString("desktopInterop"))) {
                java.io.File marker = new java.io.File(app.getFilesDir(), "desktop-video-ok");
                long until = System.currentTimeMillis() + 60000;
                while (!marker.isFile() && System.currentTimeMillis() < until) Thread.sleep(100);
                assertTrue("Desktop GStreamer viewer did not decode the phone stream", marker.isFile());
            }
        } finally {
            if (client != 0) NativeBridge.disconnect(client);
            if (decoder != null) { decoder.stop(); decoder.release(); }
            app.stopService(new android.content.Intent(app, ScreenShareService.class));
        }
    }
    @Test public void controlsPhoneOnlyAfterBothGrantsAndStopsOnRevocation() throws Exception {
        MainActivity app = activity.getActivity();
        long deadline = System.currentTimeMillis() + 60000;
        while (!RemoteControlService.available() && System.currentTimeMillis() < deadline) Thread.sleep(100);
        assertTrue("Test accessibility grant did not start the service", RemoteControlService.available());
        InstrumentationRegistry.getInstrumentation().runOnMainSync(() -> {
            app.findViewById(MainActivity.SHARE_TAB_ID).performClick();
            ((CheckBox) app.findViewById(MainActivity.SHARE_CONTROL_ID)).setChecked(true);
            app.findViewById(MainActivity.SHARE_ID).performClick();
        });
        androidx.test.uiautomator.UiDevice device = androidx.test.uiautomator.UiDevice.getInstance(InstrumentationRegistry.getInstrumentation());
        androidx.test.uiautomator.UiObject2 confirm = captureConsentAfterOptionalNotificationDenial(device);
        assertTrue("Capture consent did not appear", confirm != null); confirm.click();
        while (ScreenShareService.invitation.isEmpty() && System.currentTimeMillis() < deadline) Thread.sleep(100);
        assertTrue("Phone did not start sharing", !ScreenShareService.invitation.isEmpty());
        long client = NativeBridge.connect(ScreenShareService.invitation.replaceFirst("mightymonitor://[^/]+/", "mightymonitor://127.0.0.1:5000/"));
        java.util.concurrent.atomic.AtomicBoolean running = new java.util.concurrent.atomic.AtomicBoolean(true), available = new java.util.concurrent.atomic.AtomicBoolean(), controlling = new java.util.concurrent.atomic.AtomicBoolean();
        Thread pump = new Thread(() -> {while (running.get()) {byte[] event = NativeBridge.poll(client); if (event != null && event.length == 3 && event[0] == 6) {available.set(event[1] != 0); controlling.set(event[2] != 0);} try {Thread.sleep(5);} catch (InterruptedException stop) {return;}}}); pump.start();
        android.app.Activity target = null;
        try {
            while (!available.get() && System.currentTimeMillis() < deadline) Thread.sleep(20);
            assertTrue("Phone did not advertise the locally enabled control grant", available.get());
            NativeBridge.requestControl(client, true);
            while (!controlling.get() && System.currentTimeMillis() < deadline) Thread.sleep(20);
            assertTrue("Phone rejected control", controlling.get());
            target = InstrumentationRegistry.getInstrumentation().startActivitySync(new android.content.Intent(app, InputTestActivity.class).addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK));
            android.widget.Button button = target.findViewById(InputTestActivity.TAP_ID);
            tap(client, button);
            while (InputTestActivity.taps.get() < 1 && System.currentTimeMillis() < deadline) Thread.sleep(20);
            assertTrue("Remote gesture did not click the actual Android button", InputTestActivity.taps.get() >= 1);
            android.app.Activity testTarget=target;
            InstrumentationRegistry.getInstrumentation().runOnMainSync(()->testTarget.setRequestedOrientation(android.content.pm.ActivityInfo.SCREEN_ORIENTATION_LANDSCAPE));
            androidx.test.uiautomator.UiDevice.getInstance(InstrumentationRegistry.getInstrumentation()).setOrientationLeft();
            awaitRotation(true);
            float[] buttonPoint=capturePoint(button);
            RemoteControlService.accept(mouseMove(buttonPoint[0],buttonPoint[1]));
            // The fixed portrait capture has top/bottom bars after display rotation.
            // A click in a bar must not re-use the last valid pointer position.
            int taps=InputTestActivity.taps.get();
            RemoteControlService.accept(mouseMove(.5f,.01f));
            RemoteControlService.accept(new byte[]{2,0,1});RemoteControlService.accept(new byte[]{2,0,0});
            Thread.sleep(500);
            assertTrue("Letterbox click hit the stale last-valid Android location",InputTestActivity.taps.get()==taps);
            RemoteControlService.accept(mouseMove(buttonPoint[0],buttonPoint[1]));
            RemoteControlService.accept(new byte[]{2,0,1});RemoteControlService.accept(new byte[]{2,0,0});
            long tapDeadline=System.currentTimeMillis()+10000;
            while(InputTestActivity.taps.get()==taps && System.currentTimeMillis()<tapDeadline)Thread.sleep(20);
            assertTrue("A valid rotated pointer click stopped working",InputTestActivity.taps.get()==taps+1);
            InstrumentationRegistry.getInstrumentation().runOnMainSync(()->testTarget.setRequestedOrientation(android.content.pm.ActivityInfo.SCREEN_ORIENTATION_PORTRAIT));
            androidx.test.uiautomator.UiDevice.getInstance(InstrumentationRegistry.getInstrumentation()).setOrientationNatural();
            awaitRotation(false);deadline=System.currentTimeMillis()+60000;
            android.widget.EditText editor = target.findViewById(InputTestActivity.EDITOR_ID); tap(client, editor);
            // A prior activity can still have an input connection while the remote
            // tap is in flight. Wait for this actual editor, not just any IME client.
            while (!RemoteKeyboardService.readyFor(InputTestActivity.EDITOR_ID) && System.currentTimeMillis() < deadline) Thread.sleep(20);
            assertTrue("Remote tap did not focus the test editor and connect its keyboard", RemoteKeyboardService.readyFor(InputTestActivity.EDITOR_ID));
            String expectedText="MightyMonitor नमस्ते ✓";
            NativeBridge.typeText(client, expectedText);
            java.util.concurrent.atomic.AtomicReference<String> text = new java.util.concurrent.atomic.AtomicReference<>("");
            while (!expectedText.equals(text.get()) && System.currentTimeMillis() < deadline) {InstrumentationRegistry.getInstrumentation().runOnMainSync(() -> text.set(editor.getText().toString())); Thread.sleep(20);}
            assertTrue("Remote keyboard did not type into the actual Android editor: " + text.get(), expectedText.equals(text.get()));
            ScreenShareService.revokeControl();
            while (controlling.get() && System.currentTimeMillis() < deadline) Thread.sleep(20);
            assertTrue("Revoked phone control stayed active", !controlling.get());
            boolean rejected = false; try {NativeBridge.typeText(client, "forbidden");} catch (IllegalStateException expected) {rejected = true;}
            assertTrue("Native client accepted typing after control revocation", rejected);
            app.stopService(new android.content.Intent(app,ScreenShareService.class));
            long stopDeadline=System.currentTimeMillis()+5000;
            while((ScreenShareService.sharing || !"Sharing stopped".equals(ScreenShareService.summary)) && System.currentTimeMillis()<stopDeadline)Thread.sleep(20);
            assertTrue("Stopped sharing retained a live-session status",!ScreenShareService.sharing && "Sharing stopped".equals(ScreenShareService.summary));
        } finally {
            androidx.test.uiautomator.UiDevice rotationDevice=androidx.test.uiautomator.UiDevice.getInstance(InstrumentationRegistry.getInstrumentation());
            rotationDevice.setOrientationNatural();rotationDevice.unfreezeRotation();
            running.set(false); pump.join(1000); NativeBridge.disconnect(client);
            app.stopService(new android.content.Intent(app, ScreenShareService.class));
            if (target != null) {android.app.Activity closing = target; InstrumentationRegistry.getInstrumentation().runOnMainSync(closing::finish);}
        }
    }
    private androidx.test.uiautomator.UiObject2 captureConsentAfterOptionalNotificationDenial(androidx.test.uiautomator.UiDevice device) throws Exception {
        long deadline=System.currentTimeMillis()+25000;
        while(System.currentTimeMillis()<deadline) {
            // Exercise the optional permission's denied path on newer Android versions.
            androidx.test.uiautomator.UiObject2 deny=device.findObject(androidx.test.uiautomator.By.res("com.android.permissioncontroller", "permission_deny_button"));
            if(deny!=null){deny.click();Thread.sleep(100);continue;}
            androidx.test.uiautomator.UiObject2 capture=device.findObject(androidx.test.uiautomator.By.res("android", "button1"));
            if(capture!=null)return capture;
            Thread.sleep(100);
        }
        return null;
    }
    private static byte[] mouseMove(float x,float y){return java.nio.ByteBuffer.allocate(9).order(java.nio.ByteOrder.LITTLE_ENDIAN).put((byte)1).putFloat(x).putFloat(y).array();}
    private void awaitRotation(boolean landscape)throws Exception{
        long deadline=System.currentTimeMillis()+15000;
        java.util.concurrent.atomic.AtomicBoolean ready=new java.util.concurrent.atomic.AtomicBoolean();
        androidx.test.uiautomator.UiDevice device=androidx.test.uiautomator.UiDevice.getInstance(InstrumentationRegistry.getInstrumentation());
        while(!ready.get() && System.currentTimeMillis()<deadline){ready.set((device.getDisplayWidth()>device.getDisplayHeight())==landscape);Thread.sleep(100);}
        assertTrue("Test display did not rotate",ready.get());InstrumentationRegistry.getInstrumentation().waitForIdleSync();
    }
    private float[] capturePoint(android.view.View view){
        int[] location=new int[2];java.util.concurrent.atomic.AtomicReference<float[]> point=new java.util.concurrent.atomic.AtomicReference<>();
        InstrumentationRegistry.getInstrumentation().runOnMainSync(()->{
            view.getLocationOnScreen(location);
            androidx.test.uiautomator.UiDevice device=androidx.test.uiautomator.UiDevice.getInstance(InstrumentationRegistry.getInstrumentation());
            android.util.DisplayMetrics metrics=new android.util.DisplayMetrics();metrics.widthPixels=device.getDisplayWidth();metrics.heightPixels=device.getDisplayHeight();
            float cw=ScreenShareService.captureWidth,ch=ScreenShareService.captureHeight,scale=Math.min(cw/metrics.widthPixels,ch/metrics.heightPixels);
            point.set(new float[]{((location[0]+view.getWidth()/2f)*scale+(cw-metrics.widthPixels*scale)/2)/cw,((location[1]+view.getHeight()/2f)*scale+(ch-metrics.heightPixels*scale)/2)/ch});
        });return point.get();
    }
    private void tap(long client, android.view.View view) {float[] point=capturePoint(view);NativeBridge.pointer(client,point[0],point[1],0);NativeBridge.pointer(client,point[0],point[1],2);}
    private void saveScreenshot(String name) throws Exception {
        InstrumentationRegistry.getInstrumentation().waitForIdleSync();
        android.graphics.Bitmap bitmap = InstrumentationRegistry.getInstrumentation().getUiAutomation(android.app.UiAutomation.FLAG_DONT_SUPPRESS_ACCESSIBILITY_SERVICES).takeScreenshot();
        java.io.File directory = new java.io.File(activity.getActivity().getFilesDir(), "screenshots"); directory.mkdirs();
        try (java.io.FileOutputStream output = new java.io.FileOutputStream(new java.io.File(directory, name + ".png"))) { bitmap.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, output); }
        bitmap.recycle();
    }
}
