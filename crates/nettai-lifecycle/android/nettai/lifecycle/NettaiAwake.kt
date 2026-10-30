package nettai.lifecycle

import android.app.Activity
import android.view.WindowManager

/** The screen kept on, or let go (src/awake.rs): the window's flag, set on the activity's own thread. */
object NettaiAwake {
    @JvmStatic
    fun keepScreenOn(activity: Activity, on: Boolean) = activity.runOnUiThread {
        val flag = WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON
        if (on) activity.window.addFlags(flag) else activity.window.clearFlags(flag)
    }
}
