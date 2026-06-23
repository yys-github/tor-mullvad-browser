package org.mozilla.fenix.tor

import android.app.Application
import android.util.Log
import androidx.lifecycle.AndroidViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import org.mozilla.fenix.ext.components
import org.mozilla.geckoview.TorAndroidIntegration
import org.mozilla.geckoview.TorConnectStage
import org.mozilla.geckoview.TorConnectStageName

class ProviderStoppedViewModel(
    application: Application,
) : AndroidViewModel(application), TorAndroidIntegration.BootstrapStateChangeListener {

    private val TAG = "ProviderStoppedViewModel"

    internal val providerStoppedStateFlow: MutableStateFlow<Boolean> by lazy { MutableStateFlow(false) }

    private val _maybeConfigIssue = MutableStateFlow(false)
    internal val maybeConfigIssue: StateFlow<Boolean> = _maybeConfigIssue

    override fun onBootstrapStageChange(stage: TorConnectStage) {
        when (stage.name) {
            TorConnectStageName.ProviderStopped -> {
                Log.d(TAG, "ProviderStopped detected")
                providerStoppedStateFlow.value = true
            }
            TorConnectStageName.Start -> {
                providerStoppedStateFlow.value = false
            }
            else -> {}
        }

        _maybeConfigIssue.value = stage.providerStatus.maybeConfigIssue
    }

    init {
        getApplication<Application>().components.core.geckoRuntime.torIntegrationController.registerBootstrapStateChangeListener(this)
    }

    override fun onCleared() {
        getApplication<Application>().components.core.geckoRuntime.torIntegrationController.unregisterBootstrapStateChangeListener(this)
    }

    override fun onBootstrapProgress(progress: Double, hasWarnings: Boolean) {}

}
