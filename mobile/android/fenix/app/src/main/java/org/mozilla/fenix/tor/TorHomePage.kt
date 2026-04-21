package org.mozilla.fenix.tor

import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.paint
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.painter.BrushPainter
import androidx.compose.ui.res.colorResource
import androidx.compose.ui.res.dimensionResource
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.tooling.preview.PreviewParameter
import androidx.compose.ui.tooling.preview.PreviewParameterProvider
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.mozilla.fenix.R
import org.mozilla.fenix.home.ui.SearchBarPreview

@Composable
fun TorHomePage(
    shouldInitiallyShowPromo: MutableState<Boolean> = mutableStateOf(true),
    onClicked: () -> Unit = {},
    toolBarAtTop: Boolean = false,
    toolPair: Pair<TorCampaignViewModel.Tool, TorCampaignViewModel.Tool>,
) {
    // Will persist across a single session, but not multiple sessions.
    // Tapping the close button 'X' will hide the promo for the duration of the session
    val shouldShowPromo = rememberSaveable {
        shouldInitiallyShowPromo
    }

    // Will persist through screen rotations, but not navigations (e.g. Tap on settings -> come back will update)
    // Is expected to change with every new visit to about:tor
    val toolPair = remember {
        toolPair
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(
                top = if (toolBarAtTop) dimensionResource(R.dimen.browser_navbar_height) else 0.dp,
                bottom = if (!toolBarAtTop) dimensionResource(R.dimen.browser_navbar_height) else 0.dp,
            )
            .paint(
                BrushPainter(
                    Brush.linearGradient(
                        colors = listOf(
                            colorResource(R.color.tor_homepage_gradient_start),
                            colorResource(R.color.tor_homepage_gradient_middle),
                            colorResource(R.color.tor_homepage_gradient_end),
                        ),
                        start = Offset(0f, Float.POSITIVE_INFINITY),
                        end = Offset(Float.POSITIVE_INFINITY, 0f),
                    ),
                ),
            )
            .padding(
                start = 19.dp,
                end = 19.dp,
            )
            .verticalScroll(rememberScrollState()),
    ) {
        Spacer(modifier = Modifier.size(17.dp))
        Row(
            modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically,
        ) {
            Image(
                painter = painterResource(R.drawable.tor_browser_app_icon),
                contentDescription = null,
                Modifier.size(35.dp),
            )
            Spacer(modifier = Modifier.size(6.dp))
            Text(
                text = stringResource(R.string.app_name),
                style = TextStyle(
                    fontSize = 20.sp,
                    color = Color(0xDEFFFFFF),
                    fontWeight = FontWeight.Bold,
                ),
            )
        }
        Spacer(Modifier.weight(1f))
        if (shouldShowPromo.value) {
            CampaignBox(
                shouldShowPromo,
                onDonateButtonClicked = onClicked,
                toolPair = toolPair
            )
            Spacer(Modifier.weight(1f))
        } else {
            Text(
                // Moved from the commit 5bb3cc6b93346dabd8d46677fae7f86a8f8a4fc2
                // "[android] Modify UI/UX", and the file HomeFragment.
                // Splits by full stops or commas and puts the parts in different lines.
                // Ignoring separators at the end of the string, it is expected
                // that there are at most two parts (e.g. "Explore. Privately.").
                text = stringResource(R.string.tor_explore_privately).replace(
                    " *([.,。।]) *".toRegex(),
                    "$1\n",
                ).trim(),
                style = TextStyle(
                    color = Color(color = 0xDEFFFFFF),
                    fontSize = 40.sp,
                    textAlign = TextAlign.Start,
                ),
                modifier = Modifier.align(Alignment.CenterHorizontally),
            )
        }
        if (!shouldShowPromo.value) {
            Spacer(Modifier.weight(1f))
            Image(
                painter = painterResource(
                    id = R.drawable.ic_onion_pattern,
                ),
                contentDescription = null, Modifier.fillMaxWidth(),
            )
        }
    }
    Spacer(modifier = Modifier.size(17.dp))
}

@Composable
@Preview
/**
 * Relevant documentation
 * https://developer.android.com/develop/ui/compose/tooling/previews#preview-viewmodel
 */
private fun TorHomePagePreview(
    @PreviewParameter(
        BooleanBooleanPreviewParameterProvider::class,
    ) booleanMatrix: Pair<Boolean, Boolean>,
) {
    val toolbarAtTop = booleanMatrix.second
    Box(
        contentAlignment = if (toolbarAtTop) Alignment.TopStart else Alignment.BottomStart,
        modifier = Modifier.fillMaxSize(),
    ) {
        SearchBarPreview() // unrestricted vertically so will follow contentAlignment
        TorHomePage(
            // restricted vertically so will not follow contentAlignment
            shouldInitiallyShowPromo = mutableStateOf(booleanMatrix.first),
            toolBarAtTop = toolbarAtTop,
            toolPair = Pair(TorCampaignViewModel.toolList[0], TorCampaignViewModel.toolList[1]),
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
