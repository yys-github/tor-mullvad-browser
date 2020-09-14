package org.mozilla.fenix.tor

import android.app.Application
import android.util.Log
import android.view.ViewGroup
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.wrapContentSize
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.Observer
import androidx.navigation.fragment.NavHostFragment
import org.mozilla.fenix.HomeActivity
import org.mozilla.fenix.NavGraphDirections
import org.mozilla.fenix.R
import org.mozilla.fenix.ext.components
import org.mozilla.gecko.util.EventCallback
import org.mozilla.geckoview.CustomSecurityLevelNotificationObject

class CustomSecurityLevelViewModel(application: Application) : AndroidViewModel(application) {
    fun setUpCustomSecurityLevelObservers(homeActivity: HomeActivity, navHost: NavHostFragment) {
        val torSecurityLevelCustomObserver = Observer<CustomSecurityLevelNotificationObject> { customSecurityLevelNotification ->
            Log.d(
                "TorAndroidIntegration",
                "torSecurityLevelCustom received: $customSecurityLevelNotification",
            )

            if (customSecurityLevelNotification.isCustom) {
                getApplication<Application>().components.settings.torSecurityLevel = TorSecurityLevel.custom.level
                showSecurityWarningDialog(homeActivity, navHost, customSecurityLevelNotification.callback)
            }
        }

        val torSecurityLevelObserver = Observer<String> { securityLevel ->
            Log.d("TorAndroidIntegration", "torSecurityLevel received: $securityLevel")
            getApplication<Application>().components.settings.torSecurityLevel = TorSecurityLevel.valueOf(securityLevel).level
        }

        getApplication<Application>().components.core.geckoRuntime.torIntegrationController.isTorSecurityLevelCustom.observe(
            homeActivity,
            torSecurityLevelCustomObserver,
        )
        getApplication<Application>().components.core.geckoRuntime.torIntegrationController.torSecurityLevel.observe(
            homeActivity,
            torSecurityLevelObserver,
        )
        getApplication<Application>().components.core.geckoRuntime.torIntegrationController.fetchSecurityLevel()
    }

    private fun showSecurityWarningDialog(homeActivity: HomeActivity, navHost: NavHostFragment, callback: EventCallback) {
        homeActivity.addContentView(
            ComposeView(homeActivity).apply {
                setContent {
                    val openDialog = remember { mutableStateOf(true) }
                    if (openDialog.value) {
                        val onDismissRequest = {
                            openDialog.value = false
                            getApplication<Application>().components.core.geckoRuntime.torIntegrationController.userDismissedCustomWarning(callback)
                        }
                        Dialog(
                            onDismissRequest = onDismissRequest,
                        ) {
                            Card(
                                modifier = Modifier.padding(32.dp),
                                shape = RoundedCornerShape(16.dp),
                            ) {
                                Text(
                                    text = getApplication<Application>().getString(R.string.tor_security_level_custom_description),
                                    modifier = Modifier.padding(16.dp),
                                )
                                Row(modifier = Modifier.fillMaxWidth()) {
                                    TextButton(
                                        onClick = onDismissRequest,
                                        modifier = Modifier.wrapContentSize(),
                                    ) {
                                        Text(
                                            getApplication<Application>().getString(R.string.standard_snackbar_error_dismiss),
                                        )
                                    }
                                    Spacer(modifier = Modifier.weight(1f))
                                    TextButton(
                                        onClick = {
                                            navHost.navController.navigate(NavGraphDirections.actionGlobalTorSecurityLevelFragment())
                                            onDismissRequest()
                                        },
                                        modifier = Modifier.wrapContentSize(),
                                    ) { Text(getApplication<Application>().getString(R.string.tor_open_security_settings)) }
                                }
                            }
                        }
                    }
                }
            },
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            ),
        )
    }
}
