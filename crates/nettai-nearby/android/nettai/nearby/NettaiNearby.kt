package nettai.nearby

import android.app.Activity
import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.os.Build
import android.provider.Settings
import android.util.Log
import java.net.Inet4Address
import java.net.InetAddress

/**
 * nettai-nearby's discovery on Android (crates/nettai-nearby/src/android.rs): NsdManager browses for the devices that
 * offer, and resolves each found, one at a time (NsdManager resolves one at a time), to its host's IPv4 address, its
 * port and the TXT record's name (`n`). The app's native library calls [start], [stop] and [deviceName] through JNI
 * (the object found by the activity's class loader) and hears what is found and lost through the `external`
 * functions it registers, on NsdManager's threads.
 */
object NettaiNearby {
    private const val TAG = "nettai-nearby"

    private var manager: NsdManager? = null
    private var discovery: NsdManager.DiscoveryListener? = null
    /** The services found and not resolved yet, and whether one is being resolved. */
    private val waiting = ArrayDeque<NsdServiceInfo>()
    private var resolving = false

    /** Browse for [service] (`_nettai._tcp`), ending a browse under way. */
    @JvmStatic
    fun start(activity: Activity, service: String) {
        synchronized(this) {
            stopLocked()
            val manager = activity.getSystemService(Context.NSD_SERVICE) as NsdManager
            val listener = object : NsdManager.DiscoveryListener {
                override fun onStartDiscoveryFailed(serviceType: String, errorCode: Int) {
                    val current = synchronized(this@NettaiNearby) {
                        val current = discovery === this
                        if (current) discovery = null
                        current
                    }
                    if (current) failed("NsdManager didn't start browsing (error $errorCode)")
                }

                override fun onStopDiscoveryFailed(serviceType: String, errorCode: Int) {
                    Log.w(TAG, "stopping the browse failed (error $errorCode)")
                }

                override fun onDiscoveryStarted(serviceType: String) {}

                override fun onDiscoveryStopped(serviceType: String) {}

                override fun onServiceFound(info: NsdServiceInfo) {
                    synchronized(this@NettaiNearby) {
                        if (discovery !== this) return
                        waiting.addLast(info)
                        resolveNext()
                    }
                }

                override fun onServiceLost(info: NsdServiceInfo) {
                    synchronized(this@NettaiNearby) {
                        if (discovery !== this) return
                        waiting.removeAll { it.serviceName == info.serviceName }
                    }
                    lost(info.serviceName)
                }
            }
            this.manager = manager
            discovery = listener
            manager.discoverServices(service, NsdManager.PROTOCOL_DNS_SD, listener)
        }
    }

    /** End the browse. */
    @JvmStatic
    fun stop() {
        synchronized(this) { stopLocked() }
    }

    private fun stopLocked() {
        val listener = discovery ?: return
        discovery = null
        waiting.clear()
        try {
            manager?.stopServiceDiscovery(listener)
        } catch (e: IllegalArgumentException) {
            // (Never started: its start failed.)
        }
    }

    /** Resolve the next service waiting, unless one is being resolved (with the lock held). */
    @Suppress("DEPRECATION")
    private fun resolveNext() {
        if (resolving) return
        val info = waiting.removeFirstOrNull() ?: return
        val manager = manager ?: return
        val listener = discovery
        resolving = true
        // (resolveService is deprecated since Android 14 for registerServiceInfoCallback, which Android 8 lacks; it
        // works on all.)
        manager.resolveService(info, object : NsdManager.ResolveListener {
            override fun onResolveFailed(info: NsdServiceInfo, errorCode: Int) {
                Log.i(TAG, "resolving ${info.serviceName} failed (error $errorCode)")
                done()
            }

            override fun onServiceResolved(info: NsdServiceInfo) {
                val address = addresses(info).firstOrNull { it is Inet4Address }?.hostAddress
                val name = info.attributes["n"]?.toString(Charsets.UTF_8) ?: ""
                val current = synchronized(this@NettaiNearby) { discovery === listener }
                if (current && address != null) found(info.serviceName, name, address, info.port)
                done()
            }

            fun done() {
                synchronized(this@NettaiNearby) {
                    resolving = false
                    resolveNext()
                }
            }
        })
    }

    @Suppress("DEPRECATION")
    private fun addresses(info: NsdServiceInfo): List<InetAddress> =
        if (Build.VERSION.SDK_INT >= 34) info.hostAddresses else listOfNotNull(info.host)

    /** The device's name as its player set it (Settings, About phone), else its model. */
    @JvmStatic
    fun deviceName(activity: Activity): String =
        Settings.Global.getString(activity.contentResolver, Settings.Global.DEVICE_NAME)?.takeIf { it.isNotBlank() } ?: Build.MODEL

    @JvmStatic
    private external fun found(key: String, name: String, host: String, port: Int)

    @JvmStatic
    private external fun lost(key: String)

    @JvmStatic
    private external fun failed(why: String)
}
