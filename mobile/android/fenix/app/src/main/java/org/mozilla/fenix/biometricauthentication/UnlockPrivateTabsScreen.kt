/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

package org.mozilla.fenix.biometricauthentication

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import mozilla.components.compose.base.annotation.FlexibleWindowLightDarkPreview
import mozilla.components.compose.base.button.PrimaryButton
import mozilla.components.compose.base.button.TextButton
import org.mozilla.fenix.R
import org.mozilla.fenix.ext.isLargeWindow
import org.mozilla.fenix.theme.FirefoxTheme
import org.mozilla.fenix.theme.Theme

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp
import org.mozilla.fenix.compose.parseHtml

private const val FILL_WIDTH_LARGE_WINDOW = 0.5f
private const val FILL_WIDTH_DEFAULT = 1.0f

/**
 * A screen allowing users to unlock their private tabs.
 *
 * @param onUnlockClicked Invoked when the user taps the unlock button.
 * @param onLeaveClicked Invoked when the user taps the leave private tabs text.
 */
@Composable
internal fun UnlockPrivateTabsScreen(
    onUnlockClicked: () -> Unit,
    onLeaveClicked: () -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(FirefoxTheme.colors.layer1)
            .padding(bottom = 24.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.SpaceBetween,
    ) {
        Spacer(modifier = Modifier.height(32.dp))

        Header()

        Footer(onUnlockClicked, onLeaveClicked)

        LaunchedEffect(Unit) {
            // Record telemetry event here as
            // part of https://mozilla-hub.atlassian.net/browse/FXDROID-3385
        }
    }
}

@Composable
private fun Header() {
    Column(
        modifier = Modifier.padding(horizontal = 16.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Row (
            verticalAlignment = Alignment.CenterVertically
        ) {
            Image(
                painter = painterResource(id = R.drawable.tor_browser_app_icon),
                contentDescription = null, // decorative only.
                Modifier
                    .size(62.dp)
                    .padding(end = 10.dp),
            )
            Text(
                text = stringResource(R.string.app_name),
                color = FirefoxTheme.colors.textPrimary,
                fontWeight = FontWeight.Bold,
                fontSize = 29.sp,
                letterSpacing = 0.18.sp,
                lineHeight = 52.sp,
            )
        }

        Spacer(modifier = Modifier.height(24.dp))

        Text(
            text = stringResource(id = R.string.tor_authentication_unlock_private_tabs, stringResource(R.string.app_name)),
            color = FirefoxTheme.colors.textPrimary,
            textAlign = TextAlign.Center,
            style = FirefoxTheme.typography.headline6,
            maxLines = 1,
        )
    }
}

@Composable
private fun Footer(onUnlockClicked: () -> Unit, onLeaveClicked: () -> Unit) {
    val fillWidthFraction = if (LocalContext.current.isLargeWindow()) {
        FILL_WIDTH_LARGE_WINDOW
    } else {
        FILL_WIDTH_DEFAULT
    }

    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        modifier = Modifier
            .padding(horizontal = 16.dp)
            .fillMaxWidth(fillWidthFraction),
    ) {
        PrimaryButton(
            text = stringResource(id = R.string.pbm_authentication_unlock),
            modifier = Modifier.fillMaxWidth(),
            onClick = onUnlockClicked,
        )
    }
}

@FlexibleWindowLightDarkPreview
@Composable
private fun UnlockPrivateTabsPreview() {
    FirefoxTheme(Theme.Private) {
        UnlockPrivateTabsScreen(
            onUnlockClicked = {},
            onLeaveClicked = {},
        )
    }
}
