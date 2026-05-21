package com.privmesh.privmesh

import android.content.Intent
import android.nfc.NfcAdapter
import android.os.Build
import android.provider.Settings
import android.telephony.SubscriptionManager
import android.telephony.TelephonyManager
import android.view.WindowManager
import io.flutter.embedding.android.FlutterFragmentActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterFragmentActivity() {
    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, "privmesh/security")
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "setSecureFlag" -> {
                        val secure = call.argument<Boolean>("secure") ?: false
                        runOnUiThread {
                            if (secure) window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
                            else window.clearFlags(WindowManager.LayoutParams.FLAG_SECURE)
                        }
                        result.success(null)
                    }
                    "openBiometricSettings" -> {
                        try {
                            val intent = when {
                                Build.VERSION.SDK_INT >= Build.VERSION_CODES.R ->
                                    Intent(Settings.ACTION_BIOMETRIC_ENROLL).apply {
                                        putExtra(
                                            Settings.EXTRA_BIOMETRIC_AUTHENTICATORS_ALLOWED,
                                            // 0x000F = BIOMETRIC_WEAK | BIOMETRIC_STRONG | DEVICE_CREDENTIAL
                                            0x000F
                                        )
                                    }
                                else ->
                                    Intent(Settings.ACTION_SECURITY_SETTINGS)
                            }
                            startActivity(intent)
                        } catch (e: Exception) {
                            // Fallback to general security settings
                            try { startActivity(Intent(Settings.ACTION_SECURITY_SETTINGS)) }
                            catch (_: Exception) {}
                        }
                        result.success(null)
                    }
                    "getNfcStatus" -> {
                        val adapter = NfcAdapter.getDefaultAdapter(this)
                        val status = when {
                            adapter == null      -> "unsupported"
                            adapter.isEnabled    -> "enabled"
                            else                 -> "disabled"
                        }
                        result.success(status)
                    }
                    "openNfcSettings" -> {
                        try {
                            startActivity(Intent(Settings.ACTION_NFC_SETTINGS))
                        } catch (_: Exception) {
                            try { startActivity(Intent(Settings.ACTION_WIRELESS_SETTINGS)) }
                            catch (_: Exception) {}
                        }
                        result.success(null)
                    }
                    "getSimCards" -> {
                        try {
                            val sm = getSystemService(TELEPHONY_SUBSCRIPTION_SERVICE) as? SubscriptionManager
                            val subs = sm?.activeSubscriptionInfoList ?: emptyList()
                            val list = subs.map { sub ->
                                val tm = (getSystemService(TELEPHONY_SERVICE) as TelephonyManager)
                                    .createForSubscriptionId(sub.subscriptionId)
                                val number = try {
                                    val n = tm.line1Number
                                    if (n.isNullOrBlank()) "" else n
                                } catch (e: SecurityException) {
                                    try {
                                        val n = sub.number
                                        if (n.isNullOrBlank()) "" else n
                                    } catch (_: Exception) { "" }
                                }
                                mapOf(
                                    "slot"        to (sub.simSlotIndex + 1).toString(),
                                    "displayName" to (sub.displayName?.toString() ?: "SIM ${sub.simSlotIndex + 1}"),
                                    "carrier"     to (sub.carrierName?.toString() ?: ""),
                                    "number"      to number,
                                    "iccid"       to (sub.iccId?.let { if (it.length >= 6) "…${it.takeLast(6)}" else it } ?: ""),
                                )
                            }
                            result.success(list)
                        } catch (e: Exception) {
                            result.error("SIM_ERROR", e.message, null)
                        }
                    }
                    else -> result.notImplemented()
                }
            }
    }
}
