package nettai.app

import android.app.NativeActivity
import android.os.Bundle
import android.system.Os
import android.view.WindowManager

/**
 * The app's activity: Android's NativeActivity, which loads the library (the manifest's android.app.lib_name) and
 * runs its android_main (src/lib.rs).
 */
class NettaiActivity : NativeActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        // Slint drops its window's OpenGL surface whenever the window is hidden, which the app does while it is in
        // the background, where Android takes the surface's window away (crates/nettai/src/activity.rs). (Before the
        // library is loaded: none of its threads reads the environment yet.)
        Os.setenv("SLINT_DESTROY_WINDOW_ON_HIDE", "1", true)
        super.onCreate(savedInstanceState)
        // The keyboard comes up over the app, which keeps its size: NativeActivity's own choice shrinks the content
        // rect, the app's safe area, to what the keyboard leaves, and a phone's layout would turn to the desktop's.
        window.setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_ADJUST_NOTHING)
    }
}
