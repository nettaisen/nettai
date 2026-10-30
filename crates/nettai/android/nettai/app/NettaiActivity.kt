package nettai.app

import android.graphics.Color
import android.os.Bundle
import android.system.Os
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.View
import android.view.WindowManager
import android.view.inputmethod.EditorInfo
import androidx.activity.SystemBarStyle
import androidx.activity.enableEdgeToEdge
import androidx.core.view.WindowInsetsCompat
import com.google.androidgamesdk.GameActivity
import nettai.gamepad.NettaiGamepads

/**
 * The app's activity: Android's GameActivity, which loads the library (the manifest's android.app.lib_name) and runs
 * its android_main (crates/nettai/src/activity.rs). A GameActivity's Java side sees input before native code (a
 * NativeActivity's doesn't), so a gamepad's events go to nettai-gamepad first ([NettaiGamepads]), and winit, the
 * app's window, never makes touches of a stick's motion.
 */
class NettaiActivity : GameActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        // Slint drops its window's OpenGL surface whenever the window is hidden, which the app does while it is in
        // the background, where Android takes the surface's window away (crates/nettai/src/activity.rs). (Before the
        // library is loaded: none of its threads reads the environment yet.)
        Os.setenv("SLINT_DESTROY_WINDOW_ON_HIDE", "1", true)
        super.onCreate(savedInstanceState)
        // The app draws under the bars, which are see-through, their icons light on its dark colors; its screens
        // keep out of them (onApplyWindowInsets).
        enableEdgeToEdge(SystemBarStyle.dark(Color.TRANSPARENT), SystemBarStyle.dark(Color.TRANSPARENT))
        // The keyboard comes up over the app, which keeps its size: shrunk to what the keyboard leaves, a phone's
        // layout would turn to the desktop's.
        window.setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_ADJUST_NOTHING)
    }

    /**
     * How far the bars and the cutouts reach into the window, handed to the app as its safe area
     * (crates/nettai/src/activity.rs: `safe_area`). The surface is the whole window, so its content rect, which the
     * app read under a NativeActivity, says nothing of them.
     */
    override fun onApplyWindowInsets(view: View, insets: WindowInsetsCompat): WindowInsetsCompat {
        val safe = insets.getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout())
        safeArea(safe.left, safe.top, safe.right, safe.bottom)
        return super.onApplyWindowInsets(view, insets)
    }

    private external fun safeArea(left: Int, top: Int, right: Int, bottom: Int)

    override fun dispatchKeyEvent(event: KeyEvent): Boolean = NettaiGamepads.take(event) || super.dispatchKeyEvent(event)

    override fun dispatchGenericMotionEvent(event: MotionEvent): Boolean =
        NettaiGamepads.take(event) || super.dispatchGenericMotionEvent(event)

    /**
     * The surface the app draws on, keys going to native code as keys: its input method connection is none, so the
     * soft keyboard types key events, and GameTextInput's key listener is taken off it, so a key reaches
     * GameActivity's `onKeyDown` (and so winit, which makes them the app's keys and text, as a NativeActivity's
     * did). GameTextInput would keep them, and say the text to native code as edits, which winit 0.30 doesn't read:
     * nothing would be typed.
     */
    override fun createSurfaceView(): InputEnabledSurfaceView = KeySurface()

    private inner class KeySurface : InputEnabledSurfaceView(this@NettaiActivity) {
        init {
            setOnKeyListener(null)
        }

        override fun onCreateInputConnection(outAttrs: EditorInfo): com.google.androidgamesdk.gametextinput.InputConnection? {
            outAttrs.inputType = EditorInfo.TYPE_NULL
            return null
        }
    }
}
