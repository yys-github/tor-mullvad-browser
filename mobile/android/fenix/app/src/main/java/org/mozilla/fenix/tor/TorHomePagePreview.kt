package org.mozilla.fenix.tor

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.tooling.preview.PreviewParameterProvider
import mozilla.components.compose.base.annotation.FlexibleWindowPreviewPortraitLandscapeEnglishArabicGerman
import org.mozilla.fenix.home.ui.SearchBarPreview

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
        SearchBarPreview() // unrestricted vertically so will follow contentAlignment
        TorHomePage(
            // restricted vertically so will not follow contentAlignment
            shouldInitiallyShowPromo = booleanMatrix.first,
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
