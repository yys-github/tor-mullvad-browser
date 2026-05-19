/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

package org.mozilla.fenix.tor

import android.os.Parcelable
import kotlinx.parcelize.Parcelize

@Parcelize
@Suppress("EnumEntryName") // We need the names to be lowercase so that they match the js backend
enum class TorSecurityLevel(val level: Int) : Parcelable {
    standard(4), safer(2), safest(1), custom(-1)

}
