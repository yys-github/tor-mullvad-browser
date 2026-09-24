package org.mozilla.fenix.tor

import android.annotation.SuppressLint
import android.app.Application
import android.util.Log
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.lifecycle.AndroidViewModel
import java.text.SimpleDateFormat
import java.util.Date

class TorCampaignViewModel(application: Application) : AndroidViewModel(application) {

    val shouldInitiallyShowPromo: MutableState<Boolean> by lazy {
        mutableStateOf(shouldInitiallyShowPromo())
    }

    @SuppressLint("SimpleDateFormat")
    fun shouldInitiallyShowPromo(): Boolean {
        val dateFormat = SimpleDateFormat("yyyy-MM-dd-hh-zzz")
        // From https://gitlab.torproject.org/tpo/applications/tor-browser/-/work_items/45217#note_3462659
        val startDate = dateFormat.parse("2026-10-13-08-UTC")
        // From https://gitlab.torproject.org/tpo/applications/tor-browser/-/work_items/45217#note_3463954
        val endDate =   dateFormat.parse("2027-01-05-00-UTC")
        val currentDate = Date()

        if (currentDate.before(startDate) || currentDate.after(endDate)) {
            return false
        }
        Log.d(
            "TorCampaignViewModel",
            "org.mozilla.fenix.BuildConfig.BUILD_TYPE = ${org.mozilla.fenix.BuildConfig.BUILD_TYPE}"
        )
        return (org.mozilla.fenix.BuildConfig.BUILD_TYPE == "release") || (org.mozilla.fenix.BuildConfig.BUILD_TYPE == "debug")
    }

}
