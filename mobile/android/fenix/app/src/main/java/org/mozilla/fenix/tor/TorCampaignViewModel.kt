package org.mozilla.fenix.tor

import android.app.Application
import android.util.Log
import androidx.annotation.StringRes
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.application
import org.mozilla.fenix.R
import org.mozilla.fenix.ext.components
import java.text.SimpleDateFormat
import java.util.Date

class TorCampaignViewModel(application: Application) : AndroidViewModel(application) {

    val shouldInitiallyShowPromo: MutableState<Boolean> by lazy {
        mutableStateOf(shouldInitiallyShowPromo())
    }

    fun shouldInitiallyShowPromo(): Boolean {
        val dateFormat = SimpleDateFormat("yyyy-MM-dd-hh-zzz")
        // From https://gitlab.torproject.org/tpo/applications/tor-browser/-/work_items/44747
        val startDate = dateFormat.parse("2026-05-19-15-UTC")
        val endDate =   dateFormat.parse("2026-06-19-00-UTC")
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

    companion object {
        val toolList: List<Tool> = listOf(
            Tool(name = "Onion Browser",             description = R.string.summer_2026_funding_tool_onion_browser_description),
            Tool(name = "Quiet",                     description = R.string.summer_2026_funding_tool_quiet_description),
            Tool(name = "Ricochet Refresh",          description = R.string.summer_2026_funding_tool_ricochet_refresh_description),
            Tool(name = "SecureDrop",                description = R.string.summer_2026_funding_tool_securedrop_description),
            Tool(name = "OnionShare",                description = R.string.summer_2026_funding_tool_onionshare_description),
            Tool(name = "Digital Security Helpdesk", description = R.string.summer_2026_funding_tool_digital_security_helpdesk_description),
            Tool(name = "Paskoocheh",                description = R.string.summer_2026_funding_tool_paskoocheh_description),
            Tool(name = "Unredacted",                description = R.string.summer_2026_funding_tool_unredacted_description),
            Tool(name = "Osservatorio Nessuno",      description = R.string.summer_2026_funding_tool_osservatorio_nessuno_description),
            Tool(name = "Save",                      description = R.string.summer_2026_funding_tool_save_description),
            Tool(name = "OONI",                      description = R.string.summer_2026_funding_tool_ooni_description),
        ).shuffled()
    }

    private var toolIndex = 0
    private fun incrementToolIndex() {
        toolIndex = (toolIndex + 2) % toolList.size
    }

    data class Tool(
        val name: String,
        @param:StringRes val description: Int,
    )

    fun getToolPair() : Pair<Tool, Tool> {
        return Pair(toolList[toolIndex], toolList[(toolIndex + 1) % toolList.size]).also { incrementToolIndex() }
    }
}
