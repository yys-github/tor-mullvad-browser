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
import org.mozilla.fenix.components.metrics.AdjustThirdPartySharingController.Companion.AURA_PARTNER_ID
import org.mozilla.fenix.components.metrics.AdjustThirdPartySharingController.Companion.META_PARTNER_ID
import org.mozilla.fenix.distributions.DistributionAdjustStartupStrategy
import org.mozilla.fenix.distributions.DistributionIdManager
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
        const val CONVERSION_EVENT_1 = 1
        const val CONVERSION_EVENT_2 = 2
        const val CONVERSION_EVENT_3 = 3
        const val CONVERSION_EVENT_4 = 4
        const val CONVERSION_EVENT_5 = 5
        const val CONVERSION_EVENT_6 = 6
        const val CONVERSION_EVENT_7 = 7
        const val CONVERSION_EVENT_8 = 8
        const val CONVERSION_EVENT_9 = 9
        const val CONVERSION_EVENT_10 = 10

        /**
         * Records a glean event matching the Adjust conversion event, and sends the Adjust attribution ping.
         */
        @VisibleForTesting
        internal fun sendGleanEventAndPing(
            event: Event,
            conversionEventRecorder: ConversionEventRecorder = GleanConversionEventRecorder(),
        ) {
            /* noop */
        }

        /**
         * Sets third party sharing settings based on distribution and attribution.
         */
        @VisibleForTesting
        internal fun applyThirdPartySharingSettings(
            distribution: DistributionIdManager.Distribution,
            isUserMetaAttributed: Boolean,
            controller: ThirdPartySharingController = AdjustThirdPartySharingController(),
        ) {
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
