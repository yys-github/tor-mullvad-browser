/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

package org.mozilla.fenix.components.metrics

import android.content.Context

/**
 * Thin seam around the Adjust SDK's static entry points so that [AdjustMetricsService] can be
 * unit-tested without mocking statics.
 */
interface AdjustSdkController {
}

/**
 * [AdjustSdkController] implementation that delegates to the Adjust SDK.
 */
class DefaultAdjustSdkController : AdjustSdkController {
}
