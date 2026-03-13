package com.synch.mobile

import android.content.Intent
import android.os.Bundle
import android.util.Log
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import uniffi.synch_ffi.generateEd25519KeypairUniffi

class MainActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        
        val textView = TextView(this).apply {
            text = "Loading Synch Core..."
            textSize = 16f
            setPadding(32, 32, 32, 32)
        }
        setContentView(textView)

        try {
            val keypair = generateEd25519KeypairUniffi()
            val message = "Synch Core Initialized!\nEd25519 PubKey:\n${keypair.publicKey}"
            textView.text = message
            Log.i("SynchMobile", message)
            
            // Start the background service
            startService(Intent(this, SynchService::class.java))
            
        } catch (e: Exception) {
            val errorMsg = "Failed to initialize Synch Core: ${e.message}"
            textView.text = errorMsg
            Log.e("SynchMobile", errorMsg, e)
        }
    }
}
