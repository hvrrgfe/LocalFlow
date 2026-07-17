package com.localflow.app

import io.flutter.embedding.android.FlutterActivity
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.security.KeyStore
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey

class MainActivity : FlutterActivity() {
    companion object {
        private const val KEYSTORE_ALIAS = "localflow_master_key"
        private const val ANDROID_KEYSTORE = "AndroidKeyStore"

        /// Initialize the Android Keystore with a master key
        /// for encrypting API keys at rest.
        @JvmStatic
        fun initKeystore(): Boolean {
            return try {
                val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE)
                keyStore.load(null)

                // Check if master key already exists
                if (!keyStore.containsAlias(KEYSTORE_ALIAS)) {
                    val keyGenerator = KeyGenerator.getInstance(
                        KeyProperties.KEY_ALGORITHM_AES,
                        ANDROID_KEYSTORE
                    )
                    val spec = KeyGenParameterSpec.Builder(
                        KEYSTORE_ALIAS,
                        KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
                    )
                        .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                        .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                        .setKeySize(256)
                        .build()
                    keyGenerator.init(spec)
                    keyGenerator.generateKey()
                }
                true
            } catch (e: Exception) {
                android.util.Log.e("LocalFlow", "Keystore init failed: ${e.message}")
                false
            }
        }

        /// Check if the master key exists.
        @JvmStatic
        fun hasMasterKey(): Boolean {
            return try {
                val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE)
                keyStore.load(null)
                keyStore.containsAlias(KEYSTORE_ALIAS)
            } catch (e: Exception) {
                false
            }
        }
    }

    override fun onCreate(savedInstanceState: android.os.Bundle?) {
        super.onCreate(savedInstanceState)

        // Initialize Android Keystore on app startup
        if (!initKeystore()) {
            android.util.Log.w("LocalFlow", "Android Keystore initialization failed")
        }
    }
}