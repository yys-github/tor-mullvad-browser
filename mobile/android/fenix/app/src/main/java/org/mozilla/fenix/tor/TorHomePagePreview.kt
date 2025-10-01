package org.mozilla.fenix.tor

import android.annotation.SuppressLint
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.tooling.preview.PreviewParameterProvider
import androidx.compose.ui.tooling.preview.Preview

@SuppressLint("UnrememberedMutableState")
@FlexibleWindowPreviewPortraitLandscapeEnglishArabicGerman
@Composable
/**
 * Relevant documentation https://developer.android.com/develop/ui/compose/tooling/previews#preview-viewmodel
 */
private fun TorHomePagePreview(
    @PreviewParameter(BooleanBooleanPreviewParameterProvider::class) booleanMatrix: Pair<Boolean, Boolean>,
) {
    val toolbarAtTop = booleanMatrix.second
    Box(
        contentAlignment = if (toolbarAtTop) Alignment.Companion.TopStart else Alignment.Companion.BottomStart,
        modifier = Modifier.Companion.fillMaxSize(),
    ) {
        TorHomePage(
            shouldInitiallyShowPromo = mutableStateOf(booleanMatrix.first),
            toolBarAtTop = toolbarAtTop,
        )
    }
}

private class BooleanBooleanPreviewParameterProvider :
    PreviewParameterProvider<Pair<Boolean, Boolean>> {
    override val values: Sequence<Pair<Boolean, Boolean>>
        get() = sequenceOf(
            Pair(true, true),
            Pair(true, false),
            Pair(false, true),
            Pair(false, false),
        )
}

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
