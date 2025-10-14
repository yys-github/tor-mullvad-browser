package org.mozilla.fenix.tor

import android.util.Log
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.lifecycle.ViewModel
import java.text.SimpleDateFormat
import java.util.Date
import kotlin.getValue

class TorCampaignViewModel : ViewModel() {
    val shouldInitiallyShowPromo: MutableState<Boolean> by lazy {
        mutableStateOf(shouldInitiallyShowPromo())
    }

    fun shouldInitiallyShowPromo(): Boolean {
//        return true // uncomment to test

        val dateFormat = SimpleDateFormat("yyyy-MM-dd-hh-zzz")
        val startDate =
            dateFormat.parse("2025-10-14-15-UTC") // from https://gitlab.torproject.org/tpo/web/team/-/issues/66
        val endDate =
            dateFormat.parse("2026-01-02-00-UTC") // from https://gitlab.torproject.org/tpo/web/team/-/issues/66#note_3257224
        val currentDate = Date()

        if (currentDate.before(startDate) || currentDate.after(endDate)) {
            return false
        }

        Log.d("TorCampaignViewModel", "org.mozilla.fenix.BuildConfig.BUILD_TYPE = ${org.mozilla.fenix.BuildConfig.BUILD_TYPE}")
        if (org.mozilla.fenix.BuildConfig.BUILD_TYPE == "release") {
            return true
        }
        return false
    }
}
