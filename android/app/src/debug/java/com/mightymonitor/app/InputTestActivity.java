package com.mightymonitor.app;
/** Debug-only target for real gesture and keyboard acceptance. */
public final class InputTestActivity extends android.app.Activity {
    public static final int EDITOR_ID = 201, TAP_ID = 202;
    public static final java.util.concurrent.atomic.AtomicInteger taps = new java.util.concurrent.atomic.AtomicInteger();
    public static final java.util.concurrent.atomic.AtomicInteger downs = new java.util.concurrent.atomic.AtomicInteger(), ups = new java.util.concurrent.atomic.AtomicInteger(), cancels = new java.util.concurrent.atomic.AtomicInteger();
    @Override public boolean dispatchTouchEvent(android.view.MotionEvent event) {
        if(event.getActionMasked()==android.view.MotionEvent.ACTION_DOWN)downs.incrementAndGet();
        if(event.getActionMasked()==android.view.MotionEvent.ACTION_UP)ups.incrementAndGet();
        if(event.getActionMasked()==android.view.MotionEvent.ACTION_CANCEL)cancels.incrementAndGet();
        return super.dispatchTouchEvent(event);
    }
    @Override public void onCreate(android.os.Bundle state) {
        super.onCreate(state); taps.set(0); downs.set(0); ups.set(0); cancels.set(0);
        android.widget.LinearLayout layout = new android.widget.LinearLayout(this); layout.setOrientation(1); layout.setPadding(40,120,40,40); layout.setBackgroundColor(android.graphics.Color.WHITE);
        android.widget.Button button = new android.widget.Button(this); button.setId(TAP_ID); button.setText("Gesture target"); button.setOnClickListener(v -> taps.incrementAndGet()); layout.addView(button,new android.widget.LinearLayout.LayoutParams(-1,160));
        android.widget.EditText editor = new android.widget.EditText(this); editor.setId(EDITOR_ID); editor.setHint("Typing target"); layout.addView(editor,new android.widget.LinearLayout.LayoutParams(-1,160));
        setContentView(layout);
    }
}
