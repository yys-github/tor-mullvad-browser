/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

package org.mozilla.fenix.utils

import android.app.Notification
import android.app.PendingIntent
import android.content.Context
import androidx.core.app.NotificationCompat
import androidx.core.content.ContextCompat
import org.mozilla.fenix.R
import mozilla.components.ui.icons.R as iconsR

/**
 * Create a [Notification] with default behaviour and styling.
 * Optionally applies BigTextStyle for extended text, if specified.
 */
fun createBaseNotification(
    context: Context,
    channelId: String,
    title: String?,
    text: String,
    onClick: PendingIntent? = null,
    onDismiss: PendingIntent? = null,
    bigTextStyle: Boolean = false,
): Notification {
    return NotificationCompat.Builder(context, channelId)
        .setSmallIcon(iconsR.drawable.mozac_ic_extension_24)
        .setContentTitle(title)
        .setContentText(text)
        .setStyle(if (bigTextStyle) NotificationCompat.BigTextStyle().bigText(text) else null)
        .setBadgeIconType(NotificationCompat.BADGE_ICON_SMALL)
        .setColor(ContextCompat.getColor(context, R.color.primary_text_light_theme))
        .setPriority(NotificationCompat.PRIORITY_DEFAULT)
        .setShowWhen(false)
        .setContentIntent(onClick)
        .setDeleteIntent(onDismiss)
        .setAutoCancel(true)
        .build()
}
