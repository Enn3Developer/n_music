package com.enn3developer.n_music

import android.app.Service
import android.content.Intent
import android.os.IBinder

/**
 * DummyService is a service that acts as a placeholder when setting up content intents in notification.
 * It prevents the application from crashing when the intent is called
 */

class DummyService : Service() {
    override fun onBind(intent: Intent): IBinder? {
        return null
    }
}