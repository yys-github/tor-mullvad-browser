/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

// Copyright (c) 2020, The Tor Project, Inc.

package org.mozilla.fenix.components

import android.os.StrictMode
import android.content.Context
import kotlinx.coroutines.DelicateCoroutinesApi
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.GlobalScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.IOException
import mozilla.components.concept.engine.webextension.WebExtension
import mozilla.components.concept.engine.webextension.WebExtensionRuntime
import mozilla.components.support.webextensions.WebExtensionSupport
import mozilla.components.support.base.log.logger.Logger
import org.mozilla.fenix.ext.components
import org.mozilla.fenix.ext.settings
import org.mozilla.fenix.tor.RunOnceBootstrapped

object TorBrowserFeatures {
    private val logger = Logger("torbrowser-features")
    private const val NOSCRIPT_ID = "{73a6fe31-595d-460b-a920-fcc0f8843232}"

    private fun installNoScript(
        context: Context,
        runtime: WebExtensionRuntime,
        onSuccess: ((WebExtension) -> Unit),
        onError: ((Throwable) -> Unit)
    ) {
        /**
         * Copy the xpi from assets to cacheDir, we do not care if the file is later deleted.
         */
        val xpiName = "$NOSCRIPT_ID.xpi"
        val addonPath = context.cacheDir.resolve(xpiName)
        val policy = StrictMode.getThreadPolicy()
        try {
            context.assets.open("extensions/$xpiName")
                .use { inStream ->
                    // we don't want penaltyDeath() on disk write
                    StrictMode.setThreadPolicy(StrictMode.ThreadPolicy.LAX)

                    addonPath.outputStream().use { outStream ->
                        inStream.copyTo(outStream)
                    }
                }
        } catch (throwable: IOException) {
            onError(throwable)
            return
        } finally {
            StrictMode.setThreadPolicy(policy)
        }

        /**
         * Install with a file:// URI pointing to the temp location where the addon was copied to.
         */
        runtime.installWebExtension(
            url = addonPath.toURI().toString(),
            onSuccess = { extension ->
                runtime.setAllowedInPrivateBrowsing(
                    extension,
                    true,
                    onSuccess,
                    onError
                )
            },
            onError = { throwable -> onError(throwable) })
    }

    @OptIn(DelicateCoroutinesApi::class) // GlobalScope usage
    private fun uninstallHTTPSEverywhere(
        runtime: WebExtensionRuntime,
        onSuccess: (() -> Unit),
        onError: ((Throwable) -> Unit)
    ) {
        // Wait for WebExtensionSupport on the I/O thread to avoid deadlocks.
        GlobalScope.launch(Dispatchers.IO) {
            WebExtensionSupport.awaitInitialization()
            // Back to the main thread.
            withContext(Dispatchers.Main) {
                val extension =
                    WebExtensionSupport.installedExtensions["https-everywhere-eff@eff.org"]
                        ?: return@withContext onSuccess() // Fine, nothing to uninstall.
                runtime.uninstallWebExtension(
                    extension,
                    onSuccess = onSuccess,
                    onError = { _, throwable -> onError(throwable) }
                )
            }
        }
    }

    fun install(context: Context, runtime: WebExtensionRuntime) {
        val settings = context.settings()
        /**
         * Remove HTTPS Everywhere if we didn't yet.
         */
        if (!settings.httpsEverywhereRemoved) {
            /**
             * Ensure HTTPS-Only is enabled.
             */
            settings.shouldUseHttpsOnly = true
            settings.shouldUseHttpsOnlyInAllTabs = true
            uninstallHTTPSEverywhere(
                runtime,
                onSuccess = {
                    settings.httpsEverywhereRemoved = true
                    logger.debug("HTTPS Everywhere extension was uninstalled successfully")
                },
                onError = { throwable ->
                    logger.error("Could not uninstall HTTPS Everywhere extension", throwable)
                }
            )
        }
        /**
         *  Install NoScript if we have not done it yet for this browser version.
         *  AMO signature is checked, but automatic updates still need to be enabled.
         */
        val extensionsVersion =
            org.mozilla.geckoview.BuildConfig.MOZ_APP_VERSION + "-" +
            org.mozilla.geckoview.BuildConfig.MOZ_APP_BUILDID + "-" +
            org.mozilla.fenix.BuildConfig.VCS_HASH
        if (settings.extensionsVersion != extensionsVersion) {
            installNoScript(
                context,
                runtime,
                onSuccess = {
                    settings.extensionsVersion = extensionsVersion
                    logger.debug("NoScript extension was installed successfully")
                },
                onError = { throwable ->
                    logger.error("Could not install NoScript extension", throwable)
                }
            )
            // This covers the edge case of when noscript is installed but a newer version is available
            // i.e. A user downloads tba, but doesn't run it or update it for a while but in that time
            // a newer version of noscript is available. It also covers the test case of purposefully
            // installing an older version to test the update feature.
            context.components.torController.registerRunOnceBootstrapped(
                object : RunOnceBootstrapped {
                    override fun onBootstrapped() {
                        context.components.addonUpdater.registerForFutureUpdates(NOSCRIPT_ID)
                    }
                },
            )
        }
    }
}
