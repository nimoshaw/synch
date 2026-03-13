package com.synch.mobile

import android.app.Service
import android.content.Intent
import android.os.IBinder
import android.util.Log

class SynchService : Service() {
    
    override fun onCreate() {
        super.onCreate()
        Log.i("SynchMobile", "SynchService created.")
    }
    
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        Log.i("SynchMobile", "SynchService started. Running background operations.")
        return START_STICKY
    }
    
    override fun onBind(intent: Intent?): IBinder? {
        return null
    }
}
