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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.material.Button
import androidx.compose.material.ButtonDefaults
import androidx.compose.material.Icon
import androidx.compose.material.IconButton
import androidx.compose.material.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.MutableState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.colorResource
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.LinkAnnotation
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextLinkStyles
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import mozilla.components.ui.colors.PhotonColors
import org.mozilla.fenix.BuildConfig
import org.mozilla.fenix.R
import java.text.SimpleDateFormat
import java.util.Date


private val alternateLayoutThreshHold = 500.dp

@Composable
fun CampaignBox(shouldShowYec: MutableState<Boolean>, onDonateButtonClicked: () -> Unit) {
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
            shouldShowYec,
            onDonateButtonClicked = onDonateButtonClicked,
        )
    }
}

@Composable
private fun CampaignLayout(
    alternateLayout: Boolean,
    maxWidth: Dp,
    shouldShowYec: MutableState<Boolean>,
    onDonateButtonClicked: () -> Unit,
) {
    Column(
        modifier = Modifier
            .padding(horizontal = 22.dp)
            .fillMaxWidth(getVariableWidth(maxWidth))
            .wrapContentHeight(),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        PurpleBox(alternateLayout, shouldShowYec, onDonateButtonClicked = onDonateButtonClicked)
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
    shouldShowYec: MutableState<Boolean>,
    onDonateButtonClicked: () -> Unit,
) {
    Box(
        modifier = Modifier.background(
            colorResource(R.color.yec_background), shape = RoundedCornerShape(8.dp),
        ),
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .wrapContentHeight(),
            horizontalArrangement = Arrangement.End,
        ) {
            ExitIcon(shouldShowYec)
        }
        DynamicCampaignContent(alternateLayout, onDonateButtonClicked = onDonateButtonClicked)
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
            .padding(16.dp)
            .fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(
            modifier = Modifier.fillMaxWidth(if (alternateLayout) 0.75f else 1.0f),
            horizontalAlignment = if (alternateLayout) Alignment.Start else Alignment.CenterHorizontally,
        ) {
            Icon(shouldShow = !alternateLayout)
            TitleText(alternateLayout)
            MainText(onDonateButtonClicked)
            Row(
                horizontalArrangement = if (alternateLayout) Arrangement.Start else Arrangement.Center,
                modifier = Modifier.fillMaxWidth(),
            ) {
                DonateButton(onDonateButtonClicked = onDonateButtonClicked)
            }
        }
        Icon(shouldShow = alternateLayout)
    }
}


@Composable
private fun TitleText(alternateLayout: Boolean) {
    Text(
        text = stringResource(R.string.free_the_internet_yec),
        style = TextStyle(
            fontSize = 56.sp,
            lineHeight = 47.6.sp,
            fontFamily = FontFamily(Font(R.font.jacquard_12)),
            fontWeight = FontWeight(400),
            color = PhotonColors.White,
            textAlign = if (alternateLayout) TextAlign.Start else TextAlign.Center,
        ),
    )
}

@Composable
private fun MainText(onDonateButtonClicked: () -> Unit) {
    Column {
        BasicText(
            text = buildAnnotatedString {
                withStyle(
                    SpanStyle(
                        color = PhotonColors.LightGrey05,
                        fontSize = 14.sp,
                    ),
                ) {
                    append(
                        stringResource(
                            R.string.body1_yec, stringResource(R.string.body1_link_yec),
                        ),
                    )
                    val linkStart: Int = stringResource((R.string.body1_yec)).indexOf("%s")
                    addLink(
                        url = LinkAnnotation.Url(
                            url = stringResource(R.string.body1_link_yec),
                            styles = TextLinkStyles(
                                style = SpanStyle(colorResource(R.color.yec_green)),
                                pressedStyle = SpanStyle(PhotonColors.LightGrey05),
                            ),
                            linkInteractionListener = { onDonateButtonClicked() },
                        ),
                        start = linkStart,
                        end = linkStart + stringResource(R.string.body1_link_yec).length,
                    )
                }
            },
            modifier = Modifier
                .fillMaxWidth()
                .padding(bottom = 8.dp),
        )
        Text(
            text = stringResource(R.string.body2_yec),
            modifier = Modifier
                .fillMaxWidth()
                .padding(
                    bottom = 16.dp,
                ),
            color = PhotonColors.LightGrey05,
            fontSize = 14.sp,
            textAlign = TextAlign.Start,
        )
    }
}

@Composable
private fun DonateButton(onDonateButtonClicked: () -> Unit) {
    Button(
        onClick = onDonateButtonClicked,
        colors = ButtonDefaults.buttonColors(
            backgroundColor = colorResource(R.color.yec_green),
        ),
        shape = RoundedCornerShape(4.dp),
        contentPadding = PaddingValues(horizontal = 16.dp, vertical = 9.dp),
    ) {
        Text(
            text = stringResource(R.string.donate_now_yec),
            textAlign = TextAlign.Center,
            fontSize = 16.sp,
            fontWeight = FontWeight.SemiBold,
            color = PhotonColors.DarkGrey80,
        )
        Spacer(
            Modifier.size(4.dp),
        )
        Image(
            painterResource(R.drawable.heart),
            contentDescription = null,
        )
    }
}
