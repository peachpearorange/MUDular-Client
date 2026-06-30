package com.mudular.client;

import android.app.Activity;
import android.graphics.Color;
import android.view.View;
import android.view.Window;
import android.view.WindowInsetsController;
import android.view.WindowInsets;
import android.view.WindowManager;

public class ImmersiveHelper {
    private static boolean applied = false;

    public static void enterImmersive(Activity activity) {
        if (applied) return;
        applied = true;
        activity.runOnUiThread(() -> {
            Window window = activity.getWindow();
            window.setDecorFitsSystemWindows(false);
            window.setStatusBarColor(Color.TRANSPARENT);
            window.setNavigationBarColor(Color.TRANSPARENT);

            WindowManager.LayoutParams params = window.getAttributes();
            params.layoutInDisplayCutoutMode =
                WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES;
            window.setAttributes(params);

            WindowInsetsController controller = window.getInsetsController();
            if (controller != null) {
                controller.setSystemBarsBehavior(
                    WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE);
                controller.hide(WindowInsets.Type.systemBars());
            }

            // Re-hide bars whenever window gains focus (e.g. after keyboard interaction)
            window.getDecorView().setOnSystemUiVisibilityChangeListener(visibility -> {
                if ((visibility & View.SYSTEM_UI_FLAG_FULLSCREEN) == 0) {
                    if (controller != null) {
                        controller.hide(WindowInsets.Type.systemBars());
                    }
                }
            });
        });
    }
}
