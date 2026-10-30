package nettai.gamepad

import android.view.InputDevice
import android.view.InputEvent
import android.view.KeyEvent
import android.view.MotionEvent

/**
 * Gamepads' input, handed to nettai-gamepad (src/android.rs) before the rest of the app hears it: the activity gives
 * every key and generic motion event to [take] first.
 */
object NettaiGamepads {
    /** Whether [event] was a gamepad's, which nettai-gamepad took (the activity then passes it on to nothing else). */
    @JvmStatic
    fun take(event: InputEvent): Boolean {
        if (!event.isFromSource(InputDevice.SOURCE_GAMEPAD) && !event.isFromSource(InputDevice.SOURCE_JOYSTICK)) return false
        return when (event) {
            is KeyEvent -> when (event.action) {
                // (A held key's repeats too: nettai-gamepad has it held already, and says whether it is a pad's.)
                KeyEvent.ACTION_DOWN -> key(event.deviceId, event.keyCode, true)
                KeyEvent.ACTION_UP -> key(event.deviceId, event.keyCode, false)
                else -> false
            }
            is MotionEvent -> {
                if (event.actionMasked != MotionEvent.ACTION_MOVE) return false
                motion(
                    event.deviceId,
                    event.getAxisValue(MotionEvent.AXIS_X),
                    event.getAxisValue(MotionEvent.AXIS_Y),
                    event.getAxisValue(MotionEvent.AXIS_Z),
                    event.getAxisValue(MotionEvent.AXIS_RZ),
                    event.getAxisValue(MotionEvent.AXIS_HAT_X),
                    event.getAxisValue(MotionEvent.AXIS_HAT_Y),
                    event.getAxisValue(MotionEvent.AXIS_LTRIGGER),
                    event.getAxisValue(MotionEvent.AXIS_RTRIGGER),
                    event.getAxisValue(MotionEvent.AXIS_BRAKE),
                    event.getAxisValue(MotionEvent.AXIS_GAS),
                )
                true
            }
            else -> false
        }
    }

    /** A gamepad's key went down or up: whether it is one nettai-gamepad has (any other goes on, as a keyboard's). */
    @JvmStatic
    private external fun key(device: Int, code: Int, down: Boolean): Boolean

    /** A gamepad's or a joystick's axes as they are now. */
    @JvmStatic
    private external fun motion(
        device: Int,
        x: Float,
        y: Float,
        z: Float,
        rz: Float,
        hatX: Float,
        hatY: Float,
        ltrigger: Float,
        rtrigger: Float,
        brake: Float,
        gas: Float,
    )
}
