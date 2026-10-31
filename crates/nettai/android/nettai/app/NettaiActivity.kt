package nettai.app

import android.graphics.Color
import android.os.Bundle
import android.system.Os
import android.text.Editable
import android.text.InputType
import android.text.Selection
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.View
import android.view.WindowManager
import android.view.inputmethod.BaseInputConnection
import android.view.inputmethod.CompletionInfo
import android.view.inputmethod.EditorInfo
import androidx.activity.SystemBarStyle
import androidx.activity.enableEdgeToEdge
import androidx.core.view.WindowInsetsCompat
import com.google.androidgamesdk.GameActivity
import com.google.androidgamesdk.gametextinput.InputConnection
import com.google.androidgamesdk.gametextinput.Settings
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
     * What the soft keyboard typed, to the field being typed in (crates/nettai/src/activity.rs: `typed`): [before]
     * letters before the cursor and [after] after it erased, then [text] typed (a line break is Enter).
     */
    private external fun typed(before: Int, text: String, after: Int)

    /**
     * The surface the app draws on, and what the soft keyboard types into it ([Typing]). GameTextInput's key
     * listener is taken off it, so a key (a hardware keyboard's) reaches GameActivity's `onKeyDown`, and so winit,
     * which makes them the app's keys and text, as a NativeActivity's did.
     */
    override fun createSurfaceView(): InputEnabledSurfaceView = KeySurface()

    private inner class KeySurface : InputEnabledSurfaceView(this@NettaiActivity) {
        init {
            setOnKeyListener(null)
        }

        override fun onCreateInputConnection(outAttrs: EditorInfo): InputConnection {
            val field = textField()
            outAttrs.inputType = field.inputType
            outAttrs.imeOptions = field.imeOptions
            outAttrs.initialSelStart = 0
            outAttrs.initialSelEnd = 0
            return Typing(this)
        }
    }

    /**
     * What the keyboard is told it types into: a text field, so it composes as in any app (pinyin, kana and the rest
     * type no keys of their own); no editor of the keyboard's own over the app in landscape; the action key is Done
     * (Enter). (GameTextInput's connection, told no type, would be Android's fallback, which turns its text back
     * into keys.)
     */
    private fun textField() = EditorInfo().apply {
        inputType = InputType.TYPE_CLASS_TEXT
        imeOptions = EditorInfo.IME_ACTION_DONE or EditorInfo.IME_FLAG_NO_FULLSCREEN or EditorInfo.IME_FLAG_NO_EXTRACT_UI
    }

    /**
     * The soft keyboard's connection: its edits, said to the app ([typed]) in the order they come. The keyboard
     * edits a text of its own here (GameTextInput's, which says nothing to native code here: no listener), and the
     * field shows it as it goes: what is being composed (pinyin before its characters are chosen, a word before it
     * is done) is typed into the field, and typed over as it changes. Once nothing is being composed, what was
     * typed is the field's, and the keyboard's text is empty again: so the keyboard sees an empty field, and a
     * delete past its own text erases the field's letters. A key the keyboard sends that types or deletes is said
     * the same way, in order with the rest; any other (the arrows) is the activity's, as a hardware keyboard's.
     * (Winit 0.30 reads none of GameTextInput's state, which is why the app hears it this way.)
     */
    private inner class Typing(view: View) : InputConnection(this@NettaiActivity, view, Settings(textField(), false)) {
        /** The keyboard's text (GameTextInput's, the same throughout). */
        private val own: Editable = editable!!

        /** The keyboard's text, as the field shows it now. */
        private var shown = ""

        init {
            // (GameTextInput's connection takes the view's keys as it is made: they stay the activity's.)
            view.setOnKeyListener(null)
        }

        override fun commitText(text: CharSequence, newCursorPosition: Int): Boolean {
            // (A line break, which a single line's text drops, is Enter.)
            val lines = text.split('\n')
            edited { super.commitText(lines[0], newCursorPosition) }
            for (line in lines.drop(1)) {
                typed(0, "\n", 0)
                if (line.isNotEmpty()) edited { super.commitText(line, newCursorPosition) }
            }
            return true
        }

        override fun setComposingText(text: CharSequence, newCursorPosition: Int): Boolean =
            edited { super.setComposingText(text, newCursorPosition) }

        override fun setComposingRegion(start: Int, end: Int): Boolean = edited { super.setComposingRegion(start, end) }

        override fun finishComposingText(): Boolean = edited { super.finishComposingText() }

        override fun commitCompletion(text: CompletionInfo): Boolean = edited { super.commitCompletion(text) }

        override fun deleteSurroundingText(before: Int, after: Int): Boolean {
            val (start, end) = cursor()
            val length = own.length
            edited { super.deleteSurroundingText(before, after) }
            erasePast(before - start, after - (length - end))
            return true
        }

        override fun deleteSurroundingTextInCodePoints(before: Int, after: Int): Boolean {
            val (start, end) = cursor()
            val text = own.toString()
            edited { super.deleteSurroundingTextInCodePoints(before, after) }
            erasePast(before - text.codePointCount(0, start), after - text.codePointCount(end, text.length))
            return true
        }

        override fun performEditorAction(action: Int): Boolean {
            edited { super.finishComposingText() }
            typed(0, "\n", 0)
            return true
        }

        override fun sendKeyEvent(event: KeyEvent): Boolean {
            val down = event.action == KeyEvent.ACTION_DOWN
            val char = event.getUnicodeChar(event.metaState)
            when {
                event.keyCode == KeyEvent.KEYCODE_DEL -> if (down) deleteSurroundingText(1, 0)
                event.keyCode == KeyEvent.KEYCODE_FORWARD_DEL -> if (down) deleteSurroundingText(0, 1)
                event.keyCode == KeyEvent.KEYCODE_ENTER || event.keyCode == KeyEvent.KEYCODE_NUMPAD_ENTER ->
                    if (down) performEditorAction(EditorInfo.IME_ACTION_DONE)
                // (A letter's key: its letter, after what was being composed. A dead key's accent, flagged in the
                // top bit, is no letter.)
                char > 0 && !Character.isISOControl(char) -> if (down) {
                    edited { super.finishComposingText() }
                    commitText(String(Character.toChars(char)), 1)
                }
                else -> return this@NettaiActivity.dispatchKeyEvent(event)
            }
            return true
        }

        /** The selection in the keyboard's text: where its cursor is (none: its end). */
        private fun cursor(): Pair<Int, Int> {
            val (start, end) = Selection.getSelectionStart(own) to Selection.getSelectionEnd(own)
            return if (start < 0 || end < 0) own.length to own.length else minOf(start, end) to maxOf(start, end)
        }

        /** Letters the keyboard deletes past its own text: the field's, erased. */
        private fun erasePast(before: Int, after: Int) {
            if (before > 0 || after > 0) typed(maxOf(before, 0), "", maxOf(after, 0))
        }

        /** [edit] made to the keyboard's text, and the field brought to it. */
        private inline fun edited(edit: () -> Boolean): Boolean {
            val done = edit()
            show()
            return done
        }

        /**
         * The field brought to the keyboard's text: what it shows of it erased back to where they differ, and the
         * rest typed. Once nothing is being composed, the text is the field's alone, and the keyboard's is emptied
         * (told it, so its cursor is where the field's text ends: its start).
         */
        private fun show() {
            val text = own.toString()
            var same = 0
            while (same < shown.length && same < text.length && shown[same] == text[same]) same++
            // (Not half of a letter of two units.)
            if (same > 0 && Character.isHighSurrogate(text[same - 1])) same--
            val erase = shown.codePointCount(same, shown.length)
            val put = text.substring(same)
            if (erase > 0 || put.isNotEmpty()) typed(erase, put, 0)
            if (BaseInputConnection.getComposingSpanStart(own) >= 0) {
                shown = text
            } else {
                shown = ""
                if (own.isNotEmpty()) {
                    own.clear()
                    endBatchEdit()
                }
            }
        }
    }
}
