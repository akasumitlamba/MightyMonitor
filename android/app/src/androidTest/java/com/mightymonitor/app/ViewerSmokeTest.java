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
}
