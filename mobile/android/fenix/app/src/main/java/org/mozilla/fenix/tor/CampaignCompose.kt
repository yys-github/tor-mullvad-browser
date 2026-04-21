package org.mozilla.fenix.tor

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.wrapContentHeight
import androidx.compose.foundation.layout.wrapContentWidth
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.Button
import androidx.compose.material.ButtonDefaults
import androidx.compose.material.Icon
import androidx.compose.material.IconButton
import androidx.compose.material.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.colorResource
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.fromHtml
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import mozilla.components.ui.colors.PhotonColors
import org.mozilla.fenix.R


private val alternateLayoutThreshHold = 500.dp

@Composable
@CampaignComposePreview
fun CampaignBox(
    shouldShowPromo: MutableState<Boolean> = mutableStateOf(true),
    onDonateButtonClicked: () -> Unit = {},
    toolPair: Pair<TorCampaignViewModel.Tool, TorCampaignViewModel.Tool> = Pair(TorCampaignViewModel.toolList[0], TorCampaignViewModel.toolList[1]),
    ) {
    BoxWithConstraints(
        contentAlignment = Alignment.Center,
        modifier = Modifier
            .fillMaxWidth()
            .fillMaxHeight(),
    ) {
        val alternateLayout = this.maxWidth >= alternateLayoutThreshHold

        CampaignLayout(
            alternateLayout,
            maxWidth = this.maxWidth,
            shouldShowPromo,
            onDonateButtonClicked = onDonateButtonClicked,
            toolPair = toolPair,
        )
    }
}

@Composable
private fun CampaignLayout(
    alternateLayout: Boolean,
    maxWidth: Dp,
    shouldShowPromo: MutableState<Boolean>,
    onDonateButtonClicked: () -> Unit,
    toolPair: Pair<TorCampaignViewModel.Tool, TorCampaignViewModel.Tool>,
) {
    Column(
        modifier = Modifier
            .padding(horizontal = 22.dp)
            .fillMaxWidth(getVariableWidth(maxWidth))
            .wrapContentHeight(),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        PurpleBox(
            alternateLayout,
            shouldShowPromo,
            onDonateButtonClicked = onDonateButtonClicked,
            toolPair = toolPair,
        )
        Spacer(Modifier.size(8.dp))
        Text(
            text = stringResource(R.string.no_donation_required_yec),
            style = TextStyle(
                fontSize = 12.5.sp,
                lineHeight = 18.75.sp,
                fontFamily = FontFamily.SansSerif,
                fontWeight = FontWeight(400),
                color = PhotonColors.LightGrey05,
                textAlign = TextAlign.Center,
            ),
        )
    }
}

private fun getVariableWidth(width: Dp): Float =
    (alternateLayoutThreshHold / width).coerceIn(0.80f, 1.0f)

@Composable
private fun PurpleBox(
    alternateLayout: Boolean,
    shouldShowPromo: MutableState<Boolean>,
    onDonateButtonClicked: () -> Unit,
    toolPair: Pair<TorCampaignViewModel.Tool, TorCampaignViewModel.Tool>,
) {
    Box(
        modifier = Modifier.background(
            colorResource(mozilla.components.ui.colors.R.color.photonViolet90),
            shape = RoundedCornerShape(8.dp),
        ),
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .wrapContentHeight(),
            horizontalArrangement = Arrangement.End,
        ) {
            ExitIcon(shouldShowPromo)
        }
        DynamicCampaignContent(
            alternateLayout, onDonateButtonClicked = onDonateButtonClicked,
            toolPair = toolPair
        )
    }
}

@Composable
private fun ExitIcon(shouldShowYec: MutableState<Boolean>) {
    IconButton(
        modifier = Modifier.padding(8.dp),
        onClick = {
            shouldShowYec.value = false
        },
    ) {
        Icon(
            painter = painterResource(id = R.drawable.ic_close),
            tint = PhotonColors.White,
            contentDescription = stringResource(R.string.close_yec_button_description),
        )
    }
}


@Composable
private fun DynamicCampaignContent(
    alternateLayout: Boolean,
    onDonateButtonClicked: () -> Unit,
    toolPair: Pair<TorCampaignViewModel.Tool, TorCampaignViewModel.Tool>,
) {
    @Composable
    fun Icon(shouldShow: Boolean) {
        if (shouldShow) {
            Image(
                painterResource(R.drawable.globe_chain_burst_yec),
                contentDescription = null,
                alignment = Alignment.Center,
            )
        }
    }
    Row(
        modifier = Modifier
            .padding(start = 16.dp, top = 32.dp, end = 16.dp, bottom = 24.dp)
            .fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(
            modifier = Modifier.fillMaxWidth(if (alternateLayout) 0.75f else 1.0f),
            horizontalAlignment = if (alternateLayout) Alignment.Start else Alignment.CenterHorizontally,
        ) {
            Icon(shouldShow = !alternateLayout)
            Spacer(Modifier.size(24.dp))
            TitleText()
            Spacer(Modifier.size(16.dp))
            MainText( toolPair = toolPair )
            Spacer(Modifier.size(24.dp))
            Row(
                horizontalArrangement = if (alternateLayout) Arrangement.Start else Arrangement.Center,
                modifier = Modifier.fillMaxWidth(),
            ) {
                DonateButton(onDonateButtonClicked = onDonateButtonClicked, alternateLayout)
            }
        }
        Icon(shouldShow = alternateLayout)
    }
}


@Composable
private fun TitleText() {
    Text(
        text = stringResource(R.string.summer_2026_funding_heading),
        style = TextStyle(
            fontSize = 24.sp,
            lineHeight = 32.sp,
            fontWeight = FontWeight(400),
            color = PhotonColors.White,
            textAlign = TextAlign.Center,
            letterSpacing = 0.18.sp,
        ),
    )
}


@Composable
private fun MainText(toolPair: Pair<TorCampaignViewModel.Tool, TorCampaignViewModel.Tool>) {
    Column {
        Text(
            AnnotatedString.fromHtml(
                // Relevant documentation on HTML markup for android https://developer.android.com/guide/topics/resources/string-resource?utm_source=android-studio-app&utm_medium=app#StylingWithHTML
                stringResource(
                    R.string.summer_2026_funding_intro,
                    "<b>" + toolPair.first.name + "</b>",
                    stringResource(toolPair.first.description),
                    "<b>" + toolPair.second.name + "</b>",
                    stringResource(toolPair.second.description),
                ),
            ), style = TextStyle(
                fontSize = 14.sp,
                lineHeight = 20.sp,
                color = PhotonColors.White,
                fontWeight = FontWeight(400),
                letterSpacing = 0.25.sp,
            )
        )
        Spacer(Modifier.size(8.dp))
        Text(
            text = AnnotatedString.fromHtml(
                stringResource(
                    R.string.summer_2026_funding_outro,
                    "<b>" + stringResource(R.string.summer_2026_call_to_donate) + "</b>"
                )
            ),
            modifier = Modifier.fillMaxWidth(),
            style = TextStyle(
                fontSize = 14.sp,
                lineHeight = 20.sp,
                fontWeight = FontWeight(400),
                letterSpacing = 0.25.sp,
                color = PhotonColors.LightGrey05,
            )
        )
    }
}

@Composable
private fun DonateButton(onDonateButtonClicked: () -> Unit, alternateLayout: Boolean) {
    Button(
        onClick = onDonateButtonClicked,
        colors = ButtonDefaults.buttonColors(
            colorResource(mozilla.components.ui.colors.R.color.photonViolet60),
        ),
        shape = RoundedCornerShape(4.dp),
        contentPadding = PaddingValues(horizontal = 16.dp),
        modifier = if (alternateLayout) Modifier.wrapContentWidth() else Modifier.fillMaxWidth()
    ) {
        Image(
            painterResource(R.drawable.heart),
            contentDescription = null,
        )
        Spacer(
            Modifier.size(8.dp),
        )
        Text(
            text = stringResource(R.string.donate_now_yec),
            textAlign = TextAlign.Center,
            fontSize = 16.sp,
            fontWeight = FontWeight.SemiBold,
            color = PhotonColors.LightGrey05,
        )
    }
}



@Preview(
    name = "Small Window",
    widthDp = 400,
)
@Preview(
    name = "Medium Window",
    widthDp = 700,
)
@Preview(
    name = "Large Window",
    widthDp = 1000,
)
@Preview(
    name = "RTL Small Window",
    locale = "ar",
    widthDp = 400,
)
@Preview(
    name = "RTL Large Window",
    locale = "ar",
    widthDp = 1000
)
annotation class CampaignComposePreview
