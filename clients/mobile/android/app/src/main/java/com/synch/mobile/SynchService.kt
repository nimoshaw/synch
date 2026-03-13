package com.synch.mobile

import android.app.Service
import android.content.Intent
import android.os.IBinder
import android.util.Log

import uniffi.synch_ffi.SynchVault

class SynchService : Service() {
    
    private var vault: SynchVault? = null
    
    override fun onCreate() {
        super.onCreate()
        Log.i("SynchMobile", "SynchService created. Bootstrapping UniFFI Vault.")
        try {
            vault = SynchVault("vault-mobile-1")
            Log.i("SynchMobile", "Vault initialized. ID: ${vault?.getVaultId()}, Version: ${vault?.getVersion()}")
        } catch (e: Exception) {
            Log.e("SynchMobile", "Failed to init vault", e)
        }
    }
    
    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        Log.i("SynchMobile", "SynchService started. Applying mock update.")
        vault?.applyMockUpdate("test.txt", "Hello from Android Background Service")
        Log.i("SynchMobile", "Mock update applied. New Version: ${vault?.getVersion()}")
        return START_STICKY
    }
    
    override fun onDestroy() {
        Log.i("SynchMobile", "SynchService destroying. Handing over resources...")
        vault = null // This should trigger Rust Drop and 'Resources released' log
        super.onDestroy()
        Log.i("SynchMobile", "SynchService destroyed.")
    }

    override fun onBind(intent: Intent?): IBinder? {
        return null
    }
}
