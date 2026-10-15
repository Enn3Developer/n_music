package com.enn3developer.n_music

import android.annotation.SuppressLint
import android.content.Context
import android.media.AudioDeviceCallback
import android.media.AudioDeviceInfo
import android.media.AudioManager
import android.os.Handler
import android.os.Looper
import com.enn3developer.n_music.core.AndroidOutput

/**
 * The outputs Android lists, handed to the core: AAudio can't list them itself. The core plays
 * on the one chosen while it is there, and opens its output again whenever they change, as when
 * headphones connect. It watches for the whole process, so the core follows devices whatever
 * shows, and the list is there before anything plays.
 */
object OutputDevices {
    /** How long a burst of changes, like a headset connecting its profiles, settles first. */
    private const val SETTLE_MS = 200L

    /** What plays media: no earpiece, call audio, radio or system routes. */
    @SuppressLint("InlinedApi")
    private val Media = setOf(
        AudioDeviceInfo.TYPE_BUILTIN_SPEAKER,
        AudioDeviceInfo.TYPE_WIRED_HEADSET,
        AudioDeviceInfo.TYPE_WIRED_HEADPHONES,
        AudioDeviceInfo.TYPE_LINE_ANALOG,
        AudioDeviceInfo.TYPE_LINE_DIGITAL,
        AudioDeviceInfo.TYPE_BLUETOOTH_A2DP,
        AudioDeviceInfo.TYPE_HDMI,
        AudioDeviceInfo.TYPE_HDMI_ARC,
        AudioDeviceInfo.TYPE_HDMI_EARC,
        AudioDeviceInfo.TYPE_USB_DEVICE,
        AudioDeviceInfo.TYPE_USB_ACCESSORY,
        AudioDeviceInfo.TYPE_USB_HEADSET,
        AudioDeviceInfo.TYPE_DOCK,
        AudioDeviceInfo.TYPE_DOCK_ANALOG,
        AudioDeviceInfo.TYPE_AUX_LINE,
        AudioDeviceInfo.TYPE_HEARING_AID,
        AudioDeviceInfo.TYPE_BLE_HEADSET,
        AudioDeviceInfo.TYPE_BLE_SPEAKER,
        AudioDeviceInfo.TYPE_BLE_BROADCAST,
    )

    private val handler = Handler(Looper.getMainLooper())
    private lateinit var context: Context
    private lateinit var manager: AudioManager
    private var sent: List<AndroidOutput>? = null

    private val callback = object : AudioDeviceCallback() {
        override fun onAudioDevicesAdded(addedDevices: Array<out AudioDeviceInfo>) = settle()

        override fun onAudioDevicesRemoved(removedDevices: Array<out AudioDeviceInfo>) = settle()
    }

    private val send = Runnable { send() }

    /** Hands the core the outputs there are now, and every change after; once per process. */
    fun start(context: Context) {
        if (this::manager.isInitialized) return
        this.context = context.applicationContext
        manager = this.context.getSystemService(AudioManager::class.java)
        // Before anything plays: the core opens its first output on the chosen device.
        send()
        manager.registerAudioDeviceCallback(callback, handler)
    }

    private fun settle() {
        handler.removeCallbacks(send)
        handler.postDelayed(send, SETTLE_MS)
    }

    private fun send() {
        val devices = manager.getDevices(AudioManager.GET_DEVICES_OUTPUTS)
            .filter { it.isSink && it.type in Media }
            .map { AndroidOutput(it.id, key(it), name(it)) }
        if (devices == sent) return
        sent = devices
        CoreRepository.setOutputDevices(devices)
    }

    /** The same device across connections, which give it a new id: its type and address. */
    private fun key(device: AudioDeviceInfo) = "${device.type}:${device.address}"

    /** As the output chip names it: this phone, headphones, or the device's own name. */
    private fun name(device: AudioDeviceInfo): String = when (device.type) {
        AudioDeviceInfo.TYPE_BUILTIN_SPEAKER -> context.getString(
            if (context.resources.configuration.smallestScreenWidthDp >= 600) R.string.this_tablet else R.string.this_phone
        )
        AudioDeviceInfo.TYPE_WIRED_HEADSET, AudioDeviceInfo.TYPE_WIRED_HEADPHONES ->
            context.getString(R.string.output_headphones)
        else -> device.productName?.toString()?.takeIf { it.isNotBlank() } ?: context.getString(R.string.output_other)
    }
}
