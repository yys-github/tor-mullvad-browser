package org.mozilla.fenix.tor


import android.content.Context
import android.util.Log
import androidx.lifecycle.LifecycleCoroutineScope
import mozilla.components.browser.engine.gecko.GeckoEngine
import org.mozilla.fenix.ext.components
import org.mozilla.geckoview.TorAndroidIntegration
import org.mozilla.geckoview.TorAndroidIntegration.BootstrapStateChangeListener
import org.mozilla.geckoview.TorAndroidIntegration.TorLogListener
import org.mozilla.geckoview.TorConnectStage
import org.mozilla.geckoview.TorConnectStageName
import org.mozilla.geckoview.TorSettings
import org.mozilla.geckoview.TorSettings.BridgeBuiltinType
import org.mozilla.geckoview.TorSettings.BridgeSource

class TorControllerGV(
    private val context: Context,
) : TorController, BootstrapStateChangeListener, TorLogListener {

    private val TAG = "TorControllerGV"

    private var runOnceBootstrappedHandlers = mutableListOf<RunOnceBootstrapped>()

    override val isBootstrapped get() =
        getTorIntegration().lastKnowStage.value?.name?.isBootstrapped ?: false

    private val entries = mutableListOf<TorLog>()
    override val logEntries get() = entries

    private fun getTorIntegration(): TorAndroidIntegration {
        return (context.components.core.engine as GeckoEngine).getTorIntegrationController()
    }

    private fun getTorSettings(): TorSettings? {
        return getTorIntegration().getSettings()
    }

    // On a fresh install bridgeEnabled can be set to true without a valid bridgeSource
    // having been selected. After first use this will not happen because last selected bridge
    // will be remembered and reused.
    // However, on first use, submitting this to TorSettings is an invalid state.
    // TorSettings.sys.mjs's #cleanupSettings will remove a lone bridgeEnabled with no source
    // selected. Therefore we check and don't call setSettings if bridgeSource isn't selected
    // (when trying to enable). Disabeling is always valid.
    private var _bridgesEnabled: Boolean? = null
    override var bridgesEnabled: Boolean
        get() {
            return _bridgesEnabled ?: getTorSettings()?.bridgesEnabled ?: false
        }
        set(value) {
            _bridgesEnabled = value
            getTorSettings()?.let {
                if (!value || it.bridgesSource != BridgeSource.Invalid) {
                    it.bridgesEnabled = value
                    getTorIntegration().setSettings(it)
                }
            }
        }

    override var bridgeTransport: TorBridgeTransportConfig
        get() {
            return when (getTorSettings()?.bridgesSource) {
                BridgeSource.BuiltIn -> {
                    when (getTorSettings()?.bridgesBuiltinType) {
                        BridgeBuiltinType.Obfs4 -> TorBridgeTransportConfig.BUILTIN_OBFS4
                        BridgeBuiltinType.Meek -> TorBridgeTransportConfig.BUILTIN_MEEK
                        BridgeBuiltinType.Snowflake -> TorBridgeTransportConfig.BUILTIN_SNOWFLAKE
                        else -> TorBridgeTransportConfig.USER_PROVIDED
                    }

                }

                BridgeSource.UserProvided -> TorBridgeTransportConfig.USER_PROVIDED
                else -> TorBridgeTransportConfig.USER_PROVIDED
            }
        }
        set(value) {
            getTorSettings()?.let {
                it.bridgesEnabled = true
                if (value == TorBridgeTransportConfig.USER_PROVIDED) {
                    // NOOP: all settings will be set in call to set userProvidedBridges and submited
                    // at the same time to clear TorSettings.sys.mjs #cleanupSettings
                    return
                } else {
                    it.bridgesSource = BridgeSource.BuiltIn
                    val bbt: BridgeBuiltinType = when (value) {
                        TorBridgeTransportConfig.BUILTIN_OBFS4 -> BridgeBuiltinType.Obfs4
                        TorBridgeTransportConfig.BUILTIN_MEEK -> BridgeBuiltinType.Meek
                        TorBridgeTransportConfig.BUILTIN_SNOWFLAKE -> BridgeBuiltinType.Snowflake
                    }
                    it.bridgesBuiltinType = bbt
                }
                getTorIntegration().setSettings(it)
            }
        }

    // Currently the UI takes a user provided string and sets this in one step so there is where we
    // actually set it.bridgesSource = BridgeSource.UserProvided, not above,
    // as TorSettings.sys.mjs #cleanupSettings could reject BridgeSource.UserProvided
    // with no bridge strings
    override var userProvidedBridges: String?
        get() {
            return getTorSettings()?.let {
                if (it.bridgesSource == BridgeSource.UserProvided) {
                    return getTorSettings()?.bridgeBridgeStrings?.joinToString("\n")
                }
                return ""
            }
        }
        set(value) {
            getTorSettings()?.let {
                Log.i(TAG, "setUserProvidedBridges: '$value'")
                // Hack: we don't have validation so lets do something quick and dirty (each line has a length)
                val  userProvidedLines: Array<String> = value?.split("\n")?.filter { it.length > 4 }?.toTypedArray() ?: arrayOf<String>()
                it.bridgesSource = BridgeSource.UserProvided
                it.bridgeBridgeStrings = userProvidedLines
                getTorIntegration().setSettings(it)
            }
        }

    override fun start() {
        getTorIntegration().registerBootstrapStateChangeListener(this)
        getTorIntegration().registerLogListener(this)
    }

    override fun stop() {
        getTorIntegration().unregisterBootstrapStateChangeListener(this)
        getTorIntegration().unregisterLogListener(this)
    }

    override fun onLog(type: String?, message: String?, timestamp: String?) {
        synchronized(entries) {
            entries.add(TorLog(type ?: "null", message ?: "null", timestamp ?: "null"))
        }
    }

    override fun registerRunOnceBootstrapped(rob: RunOnceBootstrapped) {
        // TODO Remove need for this with tb-44002
        // it would be nice to have a short circuit run and don't add if already bootstrapped
        // however this calls context.components.core.engine which tries to lazy load engine
        // which causes a recursive loop. instead we should do the work in tb-44002
        // this is currently fine as there is a single use case for this called in
        // TorBrowserFeatures that is at startup
        //if (isBootstrapped) {
        //    rob.onBootstrapped()
        //    return
        //}
        synchronized(runOnceBootstrappedHandlers) {
            if (runOnceBootstrappedHandlers.contains(rob)) {
                return
            }
            runOnceBootstrappedHandlers.add(rob)
        }
    }

    override fun unregisterRunOnceBootstrapped(rob: RunOnceBootstrapped) {
        synchronized(runOnceBootstrappedHandlers) {
            if (!runOnceBootstrappedHandlers.contains(rob)) {
                return
            }
            runOnceBootstrappedHandlers.remove(rob)
        }
    }

    override fun initiateTorBootstrap(
        lifecycleScope: LifecycleCoroutineScope?,
        withDebugLogging: Boolean,
    ) {
        getTorIntegration().beginBootstrap()
    }

    override fun stopTor() {
        getTorIntegration().cancelBootstrap()
    }

    override fun shutdown() {
        getTorIntegration().shutdown()
    }

    // TorEventsBootstrapStateChangeListener
    override fun onBootstrapStageChange(stage: TorConnectStage) {
        Log.d(TAG, "onBootstrapStageChange(stage = $stage)")

        if (stage.name == TorConnectStageName.Bootstrapped) {
            synchronized(runOnceBootstrappedHandlers) {
                runOnceBootstrappedHandlers.toList().forEach {
                    it.onBootstrapped()
                    runOnceBootstrappedHandlers.remove(it)
                }
            }
        }
    }

    // TorEventsBootstrapStateChangeListener
    override fun onBootstrapProgress(progress: Double, hasWarnings: Boolean) {
        Log.d(TAG, "onBootstrapProgress(progress = $progress, hasWarnings = $hasWarnings)")
    }
}
