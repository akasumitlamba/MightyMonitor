package com.mightymonitor.app;

import android.app.Activity;
import android.graphics.Color;
import android.media.MediaCodec;
import android.media.MediaFormat;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.text.InputType;
import android.view.Gravity;
import android.view.SurfaceHolder;
import android.view.SurfaceView;
import android.view.View;
import android.widget.Button;
import android.widget.CheckBox;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.TextView;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicInteger;

/** Platform UI and video decoding only. The screen-sharing protocol lives in Rust. */
public final class MainActivity extends Activity implements SurfaceHolder.Callback {
    public static final int ADDRESS_ID = 101, TRUST_ID = 102, CONNECT_ID = 103, SHARE_TAB_ID = 104, SHARE_TRUST_ID = 105, SHARE_ID = 106;
    private final Handler handler = new Handler(Looper.getMainLooper());
    private final ExecutorService cleanup = Executors.newSingleThreadExecutor();
    private final AtomicInteger renderedFrames = new AtomicInteger();
    private SurfaceView video;
    private TextView status;
    private EditText address;
    private CheckBox trusted;
    private Button connect;
    private MediaCodec codec;
    private long handle;
    private boolean surfaceReady;
    private byte[] pendingFrame;
    private int videoWidth, videoHeight;
    private final MediaCodec.BufferInfo bufferInfo = new MediaCodec.BufferInfo();
    private final Runnable pump = new Runnable() {
        @Override public void run() {
            if (handle == 0) return;
            try {
                drainOutput();
                for (int count = 0; count < 4; count++) {
                    if (pendingFrame != null) {
                        if (!queueFrame(pendingFrame)) break;
                        pendingFrame = null;
                    }
                    byte[] event = NativeBridge.poll(handle);
                    if (event == null) break;
                    consume(event);
                    drainOutput();
                }
            } catch (Exception error) {
                disconnect("Video stopped: " + error.getMessage());
                return;
            }
            handler.postDelayed(this, 10);
        }
    };
    private LinearLayout viewPage, sharePage;
    private TextView shareStatus;
    private Button share;
    private CheckBox shareTrust;
    private final Runnable refreshSharing = new Runnable() {
        @Override public void run() {
            share.setText(ScreenShareService.sharing ? "Stop sharing" : "Share this screen");
            shareStatus.setText(ScreenShareService.summary);
            handler.postDelayed(this, 500);
        }
    };
    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        LinearLayout layout = column(); layout.setFitsSystemWindows(true);
        layout.setPadding(dp(20), dp(18), dp(20), dp(12));
        layout.setBackgroundColor(Color.rgb(247, 248, 247));
        layout.addView(text("MightyMonitor", 26, Color.rgb(30, 39, 35)));
        LinearLayout navigation = new LinearLayout(this);
        Button viewTab = new Button(this); viewTab.setText("View");
        Button shareTab = new Button(this); shareTab.setText("Share"); shareTab.setId(SHARE_TAB_ID);
        navigation.addView(viewTab, new LinearLayout.LayoutParams(0, dp(56), 1));
        navigation.addView(shareTab, new LinearLayout.LayoutParams(0, dp(56), 1)); layout.addView(navigation);
        viewPage = column(); sharePage = column();
        layout.addView(viewPage, new LinearLayout.LayoutParams(-1, 0, 1));
        layout.addView(sharePage, new LinearLayout.LayoutParams(-1, 0, 1)); sharePage.setVisibility(View.GONE);
        viewTab.setOnClickListener(v -> { viewPage.setVisibility(View.VISIBLE); sharePage.setVisibility(View.GONE); });
        shareTab.setOnClickListener(v -> { disconnect("Not connected"); viewPage.setVisibility(View.GONE); sharePage.setVisibility(View.VISIBLE); });
        address = new EditText(this); address.setId(ADDRESS_ID); address.setSingleLine(true);
        address.setHint("Device address · 192.168.1.10:5000");
        address.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS);
        address.setText(getPreferences(MODE_PRIVATE).getString("address", ""));
        viewPage.addView(address, new LinearLayout.LayoutParams(-1, dp(56)));
        trusted = new CheckBox(this); trusted.setId(TRUST_ID); trusted.setText("Trusted Wi-Fi (unencrypted)"); viewPage.addView(trusted);
        LinearLayout buttons = new LinearLayout(this);
        connect = new Button(this); connect.setId(CONNECT_ID); connect.setText("Connect"); connect.setEnabled(false); connect.setOnClickListener(v -> startViewing());
        buttons.addView(connect, new LinearLayout.LayoutParams(0, -2, 1));
        Button stop = new Button(this); stop.setText("Disconnect"); stop.setOnClickListener(v -> disconnect("Not connected")); buttons.addView(stop, new LinearLayout.LayoutParams(0, -2, 1)); viewPage.addView(buttons);
        status = text("Not connected", 14, Color.rgb(61, 91, 77)); status.setPadding(0, dp(8), 0, dp(8)); viewPage.addView(status);
        video = new SurfaceView(this); video.getHolder().addCallback(this); video.setBackgroundColor(Color.BLACK);
        video.addOnLayoutChangeListener((v,l,t,r,b,ol,ot,or,ob) -> fitVideo());
        android.widget.FrameLayout viewport = new android.widget.FrameLayout(this);
        viewport.addView(video, new android.widget.FrameLayout.LayoutParams(-1, -1, Gravity.CENTER)); viewPage.addView(viewport, new LinearLayout.LayoutParams(-1, 0, 1));
        TextView heading = text("Share this screen", 24, Color.rgb(30, 39, 35)); heading.setPadding(0, dp(24), 0, dp(12)); sharePage.addView(heading);
        sharePage.addView(text("Open MightyMonitor on your PC and enter:", 15, Color.rgb(83, 94, 88)));
        TextView ownAddress = text(ScreenShareService.localAddress(), 22, Color.rgb(30, 39, 35)); ownAddress.setTextIsSelectable(true); ownAddress.setPadding(0, dp(12), 0, dp(12)); sharePage.addView(ownAddress);
        shareTrust = new CheckBox(this); shareTrust.setId(SHARE_TRUST_ID); shareTrust.setText("Trusted Wi-Fi (unencrypted)"); sharePage.addView(shareTrust);
        share = new Button(this); share.setId(SHARE_ID); share.setText("Share this screen"); sharePage.addView(share);
        shareStatus = text(ScreenShareService.summary, 14, Color.rgb(61, 91, 77)); sharePage.addView(shareStatus);
        share.setOnClickListener(v -> {
            if (ScreenShareService.sharing) { stopService(new android.content.Intent(this, ScreenShareService.class)); return; }
            if (!shareTrust.isChecked()) { shareStatus.setText("Confirm your trusted Wi-Fi first"); return; }
            android.media.projection.MediaProjectionManager manager = getSystemService(android.media.projection.MediaProjectionManager.class);
            android.content.Intent permission = android.os.Build.VERSION.SDK_INT >= 34 ? manager.createScreenCaptureIntent(android.media.projection.MediaProjectionConfig.createConfigForDefaultDisplay()) : manager.createScreenCaptureIntent();
            startActivityForResult(permission, 200);
        });
        shareTrust.setOnCheckedChangeListener((v, checked) -> { if (!checked) stopService(new android.content.Intent(this, ScreenShareService.class)); });
        trusted.setOnCheckedChangeListener((v, checked) -> { if (!checked) disconnect("Not connected"); });
        setContentView(layout); handler.post(refreshSharing);
    }
    private LinearLayout column() { LinearLayout layout = new LinearLayout(this); layout.setOrientation(LinearLayout.VERTICAL); return layout; }
    @Override protected void onActivityResult(int request, int result, android.content.Intent data) {
        super.onActivityResult(request, result, data);
        if (request == 200 && result == RESULT_OK && data != null && shareTrust.isChecked()) {
            android.content.Intent service = new android.content.Intent(this, ScreenShareService.class).putExtra("resultCode", result).putExtra("permission", data);
            startForegroundService(service);
        } else if (request == 200) shareStatus.setText("Sharing cancelled");
    }
    private TextView text(String value, int size, int color) {
        TextView view = new TextView(this); view.setText(value); view.setTextSize(size); view.setTextColor(color); return view;
    }
    private int dp(int value) { return Math.round(value * getResources().getDisplayMetrics().density); }
    private void startViewing() {
        if (!surfaceReady || !trusted.isChecked()) { status.setText("Confirm your trusted network before connecting"); return; }
        disconnect("Connecting…");
        try {
            String endpoint = address.getText().toString().trim();
            handle = NativeBridge.connect(endpoint);
            if (handle == 0) throw new IllegalStateException("Could not start the network client");
            getPreferences(MODE_PRIVATE).edit().putString("address", endpoint).apply();
            renderedFrames.set(0);
            handler.post(pump);
        } catch (Exception | UnsatisfiedLinkError error) { disconnect("Cannot connect: " + error.getMessage()); }
    }
    private void consume(byte[] event) throws Exception {
        if (event.length == 0) throw new IllegalArgumentException("Empty network event");
        if (event[0] == 1 || event[0] == 4) {
            status.setText(new String(event, 1, event.length - 1, StandardCharsets.UTF_8));
            if (event[0] == 4) releaseDecoder();
        } else if (event[0] == 2) {
            if (event.length != 13) throw new IllegalArgumentException("Invalid video configuration");
            ByteBuffer config = ByteBuffer.wrap(event).order(ByteOrder.LITTLE_ENDIAN);
            videoWidth = config.getInt(1); videoHeight = config.getInt(5);
            int fps = config.getInt(9);
            if (videoWidth <= 0 || videoHeight <= 0 || videoWidth > 8192 || videoHeight > 8192 || fps <= 0 || fps > 240) throw new IllegalArgumentException("Invalid video size");
            releaseDecoder();
            codec = MediaCodec.createDecoderByType(MediaFormat.MIMETYPE_VIDEO_AVC);
            MediaFormat format = MediaFormat.createVideoFormat(MediaFormat.MIMETYPE_VIDEO_AVC, videoWidth, videoHeight);
            format.setInteger(MediaFormat.KEY_MAX_INPUT_SIZE, 16 * 1024 * 1024);
            codec.configure(format, video.getHolder().getSurface(), null, 0);
            codec.start();
            fitVideo();
        } else if (event[0] == 3) {
            if (event.length < 11 || codec == null) throw new IllegalArgumentException("Video arrived before configuration");
            pendingFrame = event;
        } else { throw new IllegalArgumentException("Unsupported network event"); }
    }
    private boolean queueFrame(byte[] event) {
        if (codec == null) return false;
        int input = codec.dequeueInputBuffer(0);
        if (input < 0) return false;
        ByteBuffer data = codec.getInputBuffer(input);
        if (data == null || data.capacity() < event.length - 10) throw new IllegalArgumentException("Video access unit exceeds decoder capacity");
        data.clear(); data.put(event, 10, event.length - 10);
        long timestamp = ByteBuffer.wrap(event).order(ByteOrder.LITTLE_ENDIAN).getLong(1);
        codec.queueInputBuffer(input, 0, event.length - 10, timestamp, event[9] != 0 ? MediaCodec.BUFFER_FLAG_KEY_FRAME : 0);
        return true;
    }
    private void drainOutput() {
        if (codec == null) return;
        for (int count = 0; count < 8; count++) {
            int output = codec.dequeueOutputBuffer(bufferInfo, 0);
            if (output >= 0) { codec.releaseOutputBuffer(output, true); renderedFrames.incrementAndGet(); }
            else if (output != MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) break;
        }
    }
    private void fitVideo() {
        if (videoWidth == 0 || videoHeight == 0 || video.getParent() == null) return;
        View parent = (View) video.getParent();
        int w = parent.getWidth(), h = parent.getHeight();
        if (w == 0 || h == 0) return;
        float scale = Math.min((float) w / videoWidth, (float) h / videoHeight);
        int fittedWidth = Math.max(1, Math.round(videoWidth * scale));
        int fittedHeight = Math.max(1, Math.round(videoHeight * scale));
        if (video.getLayoutParams().width == fittedWidth && video.getLayoutParams().height == fittedHeight) return;
        android.widget.FrameLayout.LayoutParams params = new android.widget.FrameLayout.LayoutParams(fittedWidth, fittedHeight, Gravity.CENTER);
        video.setLayoutParams(params);
    }
    private void releaseDecoder() {
        pendingFrame = null;
        if (codec != null) { try { codec.stop(); } catch (IllegalStateException ignored) {} codec.release(); codec = null; }
    }
    private void disconnect(String message) {
        handler.removeCallbacks(pump);
        long previous = handle; handle = 0;
        if (previous != 0) cleanup.execute(() -> NativeBridge.disconnect(previous));
        releaseDecoder();
        if (status != null) status.setText(message);
    }
    public int getRenderedFrames() { return renderedFrames.get(); }
    @Override public void surfaceCreated(SurfaceHolder holder) { surfaceReady = true; connect.setEnabled(true); }
    @Override public void surfaceChanged(SurfaceHolder holder, int format, int width, int height) { fitVideo(); }
    @Override public void surfaceDestroyed(SurfaceHolder holder) { surfaceReady = false; connect.setEnabled(false); disconnect("Display closed"); }
    @Override public void onPause() { disconnect("Not connected"); super.onPause(); }
    @Override public void onDestroy() { disconnect("Not connected"); handler.removeCallbacks(refreshSharing); cleanup.shutdown(); super.onDestroy(); }
}
