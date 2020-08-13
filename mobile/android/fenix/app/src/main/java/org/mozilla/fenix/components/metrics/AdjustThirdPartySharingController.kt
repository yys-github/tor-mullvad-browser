/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

package org.mozilla.fenix.components.metrics

/**
 * Controls third-party data sharing settings for distribution and attribution partners.
 */
interface ThirdPartySharingController {

    /**
     * Disables data sharing with Meta specifically, while leaving global sharing enabled.
     */
    fun disableMetaThirdPartySharing()

    /**
     * Enables data sharing exclusively for the given partner, disabling it for all others.
     *
     * @param partnerId The Adjust partner ID to enable sharing for.
     */
    fun enableThirdPartySharingForPartner(partnerId: String)

    /**
     * Disables data sharing with all third-party partners globally.
     */
    fun disableAllThirdPartySharing()
}

/**
 * [ThirdPartySharingController] implementation that delegates to the Adjust SDK.
 */
class AdjustThirdPartySharingController : ThirdPartySharingController {

    override fun disableMetaThirdPartySharing() {
        /* noop */
    }

    override fun enableThirdPartySharingForPartner(partnerId: String) {
        /* noop */
    }

    override fun disableAllThirdPartySharing() {
        /* noop */
    }

    companion object {
        /** Adjust partner ID for Meta. */
        const val META_PARTNER_ID = "34"

        /** Adjust partner ID for Aura. */
        const val AURA_PARTNER_ID = "802"
    }
}
