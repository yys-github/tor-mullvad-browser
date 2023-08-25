/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

package org.mozilla.fenix.tor

import androidx.lifecycle.LifecycleCoroutineScope

// Callback for function to be run one time when the system is bootstrapped and then disregarded
interface RunOnceBootstrapped {
    fun onBootstrapped()
}

interface TorController {
    val logEntries: MutableList<TorLog>
    val isBootstrapped: Boolean
    var bridgesEnabled: Boolean
    var bridgeTransport: TorBridgeTransportConfig
    var userProvidedBridges: String?

    fun start()
    fun stop()

    // TorBrowserFeatures.install wants to register a callback for when tor bootstraps the first time
    // so it can then check for noscript updates.
    // Currently it needs to register it before TorAndroidIntegration is fully loaded, so this way
    // they can register with TorController which will start streaming events from TAS when available
    // and call them one time when the system is bootstrapped
    // TODO: rewire the noscript update call in TorBrowserFeatures.install
    //   a) call TorBrowserFeatures.install from somewhere else (ex: move from Core.GeckoEngine.also
    //      to maybe FenixApplication.setupInMainProcessOnly
    //      dan: had trouble with this first time:
    //      https://gitlab.torproject.org/tpo/applications/tor-browser/-/merge_requests/1423#note_3191590
    //   b) just move the call to `context.components.addonUpdater.update(NOSCRIPT_ID)` somewhere else
    //      that can use TorAndroidIntegration.BootstrapListener
    fun registerRunOnceBootstrapped(rob: RunOnceBootstrapped)
    fun unregisterRunOnceBootstrapped(rob: RunOnceBootstrapped)

    fun initiateTorBootstrap(lifecycleScope: LifecycleCoroutineScope? = null, withDebugLogging: Boolean = false)
    fun stopTor()
}
