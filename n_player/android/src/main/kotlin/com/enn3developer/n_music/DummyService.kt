package com.enn3developer.n_music

import android.app.Service
import android.content.Intent
import android.os.IBinder

class DummyService : Service() {
    override fun onBind(intent: Intent): IBinder? {
        return null
    }
}