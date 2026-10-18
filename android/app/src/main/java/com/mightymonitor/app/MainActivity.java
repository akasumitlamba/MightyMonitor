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
    public static final int ADDRESS_ID = 101, TRUST_ID = 102, CONNECT_ID = 103;
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
    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        LinearLayout layout = new LinearLayout(this);
        layout.setOrientation(LinearLayout.VERTICAL);
        layout.setFitsSystemWindows(true);
        layout.setPadding(dp(18), dp(16), dp(18), dp(12));
        layout.setBackgroundColor(Color.rgb(16, 39, 72));
        TextView title = text("MightyMonitor", 25, Color.WHITE);
        layout.addView(title);
        layout.addView(text("View your PC screen on this device", 14, Color.rgb(190, 207, 234)));
        address = new EditText(this);
        address.setId(ADDRESS_ID);
        address.setSingleLine(true);
        address.setTextColor(Color.WHITE);
        address.setHintTextColor(Color.rgb(166, 185, 214));
        address.setHint("PC IP:port, e.g. 192.168.1.10:5000");
        address.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS);
        address.setText(getPreferences(MODE_PRIVATE).getString("address", ""));
        layout.addView(address, new LinearLayout.LayoutParams(-1, dp(52)));
        trusted = new CheckBox(this);
        trusted.setId(TRUST_ID);
        trusted.setText("I’m using a trusted local network");
        trusted.setTextColor(Color.WHITE);
        layout.addView(trusted);
        layout.addView(text("Alpha: viewing only. Connections are not encrypted or password protected.", 12, Color.rgb(190, 207, 234)));
        LinearLayout buttons = new LinearLayout(this);
        connect = new Button(this); connect.setId(CONNECT_ID); connect.setText("Connect"); connect.setEnabled(false);
        connect.setOnClickListener(view -> startViewing());
        buttons.addView(connect, new LinearLayout.LayoutParams(0, -2, 1));
        Button stop = new Button(this); stop.setText("Disconnect"); stop.setOnClickListener(view -> disconnect("Not connected"));
        buttons.addView(stop, new LinearLayout.LayoutParams(0, -2, 1));
        layout.addView(buttons);
        status = text("Not connected", 13, Color.rgb(161, 231, 212));
        status.setPadding(0, dp(9), 0, dp(9));
        layout.addView(status);
        video = new SurfaceView(this); video.getHolder().addCallback(this);
        video.setBackgroundColor(Color.BLACK);
        video.addOnLayoutChangeListener((v,l,t,r,b,ol,ot,or,ob) -> fitVideo());
        android.widget.FrameLayout viewport = new android.widget.FrameLayout(this);
        viewport.addView(video, new android.widget.FrameLayout.LayoutParams(-1, -1, Gravity.CENTER));
        layout.addView(viewport, new LinearLayout.LayoutParams(-1, 0, 1));
        setContentView(layout);
        trusted.setOnCheckedChangeListener((button, checked) -> { if (!checked) disconnect("Not connected"); });
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
    @Override public void onDestroy() { disconnect("Not connected"); cleanup.shutdown(); super.onDestroy(); }
}
