package com.enn3developer.n_music.ui.player

import android.annotation.SuppressLint
import android.media.AudioAttributes
import android.media.AudioDeviceCallback
import android.media.AudioDeviceInfo
import android.media.AudioManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalWindowInfo
import androidx.compose.ui.res.stringResource
import com.enn3developer.n_music.R

/**
 * Bluetooth and the like, then wired, USB and HDMI: what Android 11 and 12 most likely play on.
 * Android 11 reports none of the types Android 12 added, so there they just never match.
 */
@SuppressLint("InlinedApi")
private val Ranked = listOf(
    setOf(
        AudioDeviceInfo.TYPE_BLUETOOTH_A2DP,
        AudioDeviceInfo.TYPE_BLE_HEADSET,
        AudioDeviceInfo.TYPE_BLE_SPEAKER,
        AudioDeviceInfo.TYPE_HEARING_AID,
    ),
    setOf(AudioDeviceInfo.TYPE_WIRED_HEADSET, AudioDeviceInfo.TYPE_WIRED_HEADPHONES),
    setOf(AudioDeviceInfo.TYPE_USB_HEADSET, AudioDeviceInfo.TYPE_USB_DEVICE, AudioDeviceInfo.TYPE_USB_ACCESSORY),
    setOf(AudioDeviceInfo.TYPE_HDMI, AudioDeviceInfo.TYPE_HDMI_ARC, AudioDeviceInfo.TYPE_HDMI_EARC),
)

/** The device media plays on now; `null` for the device's own speaker. */
private fun mediaOutput(manager: AudioManager): AudioDeviceInfo? {
    val outputs = manager.getDevices(AudioManager.GET_DEVICES_OUTPUTS)
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
        val media = AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_MEDIA).build()
        val routed = manager.getAudioDevicesForAttributes(media).firstOrNull() ?: return null
        return outputs.firstOrNull { it.type == routed.type && it.address == routed.address }
            ?: outputs.firstOrNull { it.type == routed.type }
    }
    return Ranked.firstNotNullOfOrNull { types -> outputs.firstOrNull { it.type in types } }
}

/**
 * What the output chip says media plays on: headphones' or a speaker's own name, Headphones for
 * wired ones, or this phone. It follows devices coming and going, and checks again whenever the
 * window gets back its focus, as after Android's output switcher.
 */
@Composable
fun rememberOutputName(): String {
    val context = LocalContext.current
    val manager = remember(context) { context.getSystemService(AudioManager::class.java) }
    var device by remember(manager) { mutableStateOf(mediaOutput(manager)) }
    DisposableEffect(manager) {
        val callback = object : AudioDeviceCallback() {
            override fun onAudioDevicesAdded(added: Array<out AudioDeviceInfo>) {
                device = mediaOutput(manager)
            }

            override fun onAudioDevicesRemoved(removed: Array<out AudioDeviceInfo>) {
                device = mediaOutput(manager)
            }
        }
        manager.registerAudioDeviceCallback(callback, Handler(Looper.getMainLooper()))
        onDispose { manager.unregisterAudioDeviceCallback(callback) }
    }
    val focused = LocalWindowInfo.current.isWindowFocused
    LaunchedEffect(focused) { if (focused) device = mediaOutput(manager) }
    val own = if (LocalConfiguration.current.smallestScreenWidthDp >= 600) R.string.this_tablet else R.string.this_phone
    val playing = device ?: return stringResource(own)
    return when (playing.type) {
        AudioDeviceInfo.TYPE_BUILTIN_SPEAKER,
        AudioDeviceInfo.TYPE_BUILTIN_SPEAKER_SAFE,
        AudioDeviceInfo.TYPE_BUILTIN_EARPIECE,
        -> stringResource(own)
        AudioDeviceInfo.TYPE_WIRED_HEADSET, AudioDeviceInfo.TYPE_WIRED_HEADPHONES -> stringResource(R.string.output_headphones)
        else -> playing.productName?.toString()?.takeIf { it.isNotBlank() } ?: stringResource(R.string.output_other)
    }
}
