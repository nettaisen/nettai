package nettai.updater

import android.app.Activity
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageInstaller
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.content.ContextCompat
import java.io.File

/**
 * The app's update put in place by the system's installer (crates/nettai-updater/src/platform/android.rs): the APK the
 * app downloaded and checked, written into a session of Android's [PackageInstaller] that asks for no confirmation (the
 * app updates itself: Android 12 and later install it without a word, with the manifest's
 * UPDATE_PACKAGES_WITHOUT_USER_ACTION), and committed. Android ends the app once the update is in place. Where it asks
 * the player first (Settings' "Install unknown apps" not yet allowed for the app; Android 11 and earlier, always), its
 * answer says so and carries the screen that asks, which the activity shows; any other answer is the app's, through
 * [ended].
 */
object NettaiUpdater {
    /** The installer's answers, to this app alone. */
    private const val ANSWER = "nettai.updater.ANSWER"

    /** The installed app's version (the APK's versionName). */
    @JvmStatic
    fun version(activity: Activity): String {
        val info = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            activity.packageManager.getPackageInfo(activity.packageName, PackageManager.PackageInfoFlags.of(0))
        } else {
            @Suppress("DEPRECATION")
            activity.packageManager.getPackageInfo(activity.packageName, 0)
        }
        return info.versionName ?: ""
    }

    /** Install [apk] over the app: null once the session is committed, else why it couldn't be. */
    @JvmStatic
    fun install(activity: Activity, apk: String): String? {
        val installer = activity.packageManager.packageInstaller
        val file = File(apk)
        val params = PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL).apply {
            setAppPackageName(activity.packageName)
            setSize(file.length())
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                setRequireUserAction(PackageInstaller.SessionParams.USER_ACTION_NOT_REQUIRED)
            }
        }
        var id = -1
        return try {
            id = installer.createSession(params)
            installer.openSession(id).use { session ->
                file.inputStream().use { input ->
                    session.openWrite("nettai.apk", 0, file.length()).use { output ->
                        input.copyTo(output)
                        session.fsync(output)
                    }
                }
                ContextCompat.registerReceiver(activity, Answer(activity), IntentFilter(ANSWER), ContextCompat.RECEIVER_NOT_EXPORTED)
                // (Mutable: the installer adds its answer to it, which Android 12 and later must be told. To this app
                // alone: the receiver isn't exported.)
                val answer = Intent(ANSWER).setPackage(activity.packageName)
                val mutable = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) PendingIntent.FLAG_MUTABLE else 0
                val flags = PendingIntent.FLAG_UPDATE_CURRENT or mutable
                session.commit(PendingIntent.getBroadcast(activity, id, answer, flags).intentSender)
            }
            null
        } catch (e: Exception) {
            if (id != -1) {
                runCatching { installer.abandonSession(id) }
            }
            e.toString()
        }
    }

    /** The installer's answer: the screen that asks the player, shown; or the end, said to the app. */
    private class Answer(private val activity: Activity) : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            val status = intent.getIntExtra(PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_FAILURE)
            if (status == PackageInstaller.STATUS_PENDING_USER_ACTION) {
                val asking = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                    intent.getParcelableExtra(Intent.EXTRA_INTENT, Intent::class.java)
                } else {
                    @Suppress("DEPRECATION")
                    intent.getParcelableExtra(Intent.EXTRA_INTENT)
                }
                if (asking != null) {
                    activity.startActivity(asking)
                    return
                }
            }
            activity.unregisterReceiver(this)
            // (Success never comes here: Android ends the app as it installs the update.)
            if (status != PackageInstaller.STATUS_SUCCESS) {
                ended(status, intent.getStringExtra(PackageInstaller.EXTRA_STATUS_MESSAGE) ?: "")
            }
        }
    }

    /** The installer's answer where the update wasn't installed: its status, and its message. */
    @JvmStatic
    private external fun ended(status: Int, message: String)
}
