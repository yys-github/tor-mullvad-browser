package org.mozilla.fenix.tor

import androidx.compose.foundation.Image
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
import androidx.compose.runtime.mutableStateOf
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
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import mozilla.components.compose.base.annotation.FlexibleWindowLightDarkPreview
import org.mozilla.fenix.R

@Composable
@FlexibleWindowLightDarkPreview
fun TorHomePage(
    shouldInitiallyShowPromo: Boolean = false,
    onClicked: () -> Unit = {},
    toolBarAtTop: Boolean = true,
) {
    val shouldShowPromo = rememberSaveable {
        mutableStateOf(shouldInitiallyShowPromo)
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
            CampaignBox(shouldShowPromo, onDonateButtonClicked = onClicked)
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
            Spacer(Modifier.weight(1f))
            Image(
                painter = painterResource(
                    id = R.drawable.ic_onion_pattern,
                ),
                contentDescription = null, Modifier.fillMaxWidth(),
            )
        }
        Spacer(modifier = Modifier.size(17.dp))
    }
}
