/* This Source Code Form is subject to the terms of the Mozilla Public
* License, v. 2.0. If a copy of the MPL was not distributed with this
* file, You can obtain one at http://mozilla.org/MPL/2.0/. */

package mozilla.components.compose.base.annotation

import androidx.compose.ui.tooling.preview.Preview

/**
 * A wrapper annotation for creating a preview that renders a preview for each value of portrait and landscape for english, arabic, and german.
 */
@Preview(
    name = "mobile portrait",
    group = "english",
    widthDp = 360,
    heightDp = 780,
    locale = "en",
    device = "id:pixel_5",
)
@Preview(
    name = "mobile landscape",
    group = "english",
    widthDp = 780,
    heightDp = 360,
    locale = "en",
    device = "spec:parent=pixel_5,orientation=landscape",
)
@Preview(
    name = "mobile portrait",
    group = "arabic",
    widthDp = 360,
    heightDp = 780,
    locale = "ar",
    device = "id:pixel_5",
)
@Preview(
    name = "mobile landscape",
    group = "arabic",
    widthDp = 780,
    heightDp = 360,
    locale = "ar",
    device = "spec:parent=pixel_5,orientation=landscape",
)
@Preview(
    name = "mobile portrait",
    group = "german",
    widthDp = 360,
    heightDp = 780,
    locale = "de",
    device = "id:pixel_5",
)
@Preview(
    name = "mobile landscape",
    group = "german",
    widthDp = 780,
    heightDp = 360,
    locale = "de",
    device = "spec:parent=pixel_5,orientation=landscape",
)
annotation class FlexibleWindowPreviewPortraitLandscapeEnglishArabicGerman
