/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

package org.mozilla.fenix.components.metrics

import android.app.Application
import androidx.annotation.VisibleForTesting
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import mozilla.components.lib.crash.CrashReporter
import mozilla.components.support.base.log.logger.Logger
import org.mozilla.fenix.BuildConfig
import org.mozilla.fenix.Config
import org.mozilla.fenix.GleanMetrics.AdjustAttribution
import org.mozilla.fenix.GleanMetrics.Pings
import org.mozilla.fenix.distributions.DistributionAdjustStartupStrategy
import org.mozilla.fenix.ext.components
import org.mozilla.fenix.utils.Settings

class AdjustMetricsService(
    private val application: Application,
    private val storage: MetricsStorage,
    private val crashReporter: CrashReporter,
    private val dispatcher: CoroutineDispatcher = Dispatchers.IO,
) : MetricsService {
    override val type = MetricServiceType.Marketing
    private val logger = Logger("AdjustMetricsService")

    @Suppress("CognitiveComplexMethod")
    override fun start() {
        /* noop */
    }

    override fun stop() {
        /* noop */
    }

    @Suppress("TooGenericExceptionCaught")
    override fun track(event: Event) {
        /* noop */
    }

    override fun shouldTrack(event: Event): Boolean =
        event is Event.GrowthData || event is Event.FirstWeekPostInstall

    companion object {
        const val META_PARTNER_ID = "34"

        private fun enableOnlyMetaThirdPartySharing() {
            /* noop */
        }

        private fun disableMetaThirdPartySharing() {
            /* noop */
        }

        @VisibleForTesting
        internal fun alreadyKnown(settings: Settings): Boolean {
            /* noop */
            return false
        }

        private fun triggerPing() {
            /* noop */
        }
    }
}
