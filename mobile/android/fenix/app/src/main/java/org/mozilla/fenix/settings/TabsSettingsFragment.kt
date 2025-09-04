/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

package org.mozilla.fenix.settings

import android.os.Bundle
import android.view.View
import androidx.preference.PreferenceCategory
import androidx.preference.PreferenceFragmentCompat
import androidx.preference.SwitchPreference
import mozilla.telemetry.glean.private.NoExtras
import org.mozilla.fenix.GleanMetrics.Events
import org.mozilla.fenix.GleanMetrics.Tabs
import org.mozilla.fenix.R
import org.mozilla.fenix.ext.settings
import org.mozilla.fenix.ext.showToolbar
import org.mozilla.fenix.utils.view.addToRadioGroup

import android.content.Intent
import android.provider.Settings
import androidx.activity.result.ActivityResultLauncher
import androidx.biometric.BiometricManager
import androidx.preference.Preference
import org.mozilla.fenix.ext.registerForActivityResult
import org.mozilla.fenix.settings.biometric.DefaultBiometricUtils
import org.mozilla.fenix.settings.biometric.ext.isAuthenticatorAvailable
import org.mozilla.fenix.settings.biometric.ext.isHardwareAvailable

/**
 * Lets the user customize auto closing tabs.
 */
class TabsSettingsFragment : PreferenceFragmentCompat() {
    private lateinit var listRadioButton: RadioButtonPreference
    private lateinit var gridRadioButton: RadioButtonPreference
    private lateinit var radioManual: RadioButtonPreference
    private lateinit var radioOneDay: RadioButtonPreference
    private lateinit var radioOneWeek: RadioButtonPreference
    private lateinit var radioOneMonth: RadioButtonPreference
    private lateinit var inactiveTabsCategory: PreferenceCategory
    private lateinit var inactiveTabs: SwitchPreference

    override fun onCreatePreferences(savedInstanceState: Bundle?, rootKey: String?) {
        setPreferencesFromResource(R.xml.tabs_preferences, rootKey)

        findPreference<RadioButtonPreference>(getString(R.string.pref_key_close_tabs_manually))?.parent?.apply {
            isVisible = !context.settings().shouldDisableNormalMode
        }

        findPreference<PreferenceCategory>(getString(R.string.pref_key_inactive_tabs_category))?.apply {
            isVisible = !context.settings().shouldDisableNormalMode
        }

        startForResult = registerForActivityResult(
            onFailure = { },
            onSuccess = { onSuccessfulAuthenticationUsingFallbackPrompt() },
        )
    }

    private lateinit var startForResult: ActivityResultLauncher<Intent>

    private fun onSuccessfulAuthenticationUsingFallbackPrompt() {
        val newValue = !requireContext().settings().privateBrowsingLockedEnabled
        requireContext().settings().privateBrowsingLockedEnabled = newValue
        // Update switch state manually
        requirePreference<SwitchPreference>(R.string.pref_key_private_browsing_locked_enabled).apply {
            isChecked = !isChecked
        }
    }

    private fun onSuccessfulAuthenticationUsingPrimaryPrompt(
        pbmLockEnabled: Boolean,
        preference: Preference,
    ) {
        requireContext().settings().privateBrowsingLockedEnabled = pbmLockEnabled
        // Update switch state manually
        (preference as? SwitchPreference)?.isChecked = pbmLockEnabled
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, savedInstanceState)
        Tabs.settingOpened.record(NoExtras())
    }

    override fun onResume() {
        super.onResume()
        showToolbar(getString(R.string.preferences_tabs))

        setupPreferences()
    }

    private fun setupPreferences() {
        // This should be the only use case for pref_key_tab_view_list_do_not_use.
        // In the Fenix impl of RadioGroups, we need to always have an individual pref value
        // for it to work. This is weird for a radio group that should hold a value from that group.
        // For the tabs tray, we only need a boolean value, so let's rely on only the
        // pref_key_tab_view_grid and look into using the native RadioGroup in the future.
        listRadioButton = requirePreference(R.string.pref_key_tab_view_list_do_not_use)
        gridRadioButton = requirePreference(R.string.pref_key_tab_view_grid)

        radioManual = requirePreference(R.string.pref_key_close_tabs_manually)
        radioOneMonth = requirePreference(R.string.pref_key_close_tabs_after_one_month)
        radioOneWeek = requirePreference(R.string.pref_key_close_tabs_after_one_week)
        radioOneDay = requirePreference(R.string.pref_key_close_tabs_after_one_day)

        inactiveTabs = requirePreference<SwitchPreference>(R.string.pref_key_inactive_tabs).also {
            it.isChecked = requireContext().settings().inactiveTabsAreEnabled
            it.onPreferenceChangeListener = SharedPreferenceUpdater()
        }

        inactiveTabsCategory = requirePreference<PreferenceCategory>(R.string.pref_key_inactive_tabs_category).also {
            it.isEnabled = !(it.context.settings().closeTabsAfterOneDay || it.context.settings().closeTabsAfterOneWeek)
        }

        listRadioButton.onClickListener(::sendTabViewTelemetry)
        gridRadioButton.onClickListener(::sendTabViewTelemetry)

        radioManual.onClickListener(::enableInactiveTabsSetting)
        radioOneDay.onClickListener(::disableInactiveTabsSetting)
        radioOneWeek.onClickListener(::disableInactiveTabsSetting)
        radioOneMonth.onClickListener(::enableInactiveTabsSetting)

        setupRadioGroups()
        /**
         * Changes in this file for "tor-browser#44027 Update PBM lockscreen" were copied from
         * [PrivateBrowsingFragment] and changed to make sense and work for TBA such as removing
         * any use of nimbus/glean that was being used for business logic which was making the
         * release build variant not work. We should check [PrivateBrowsingFragment] for updates
         * when we rebase
         * */
        setUpHideBrowsingSessionPreference()
    }

    private fun setUpHideBrowsingSessionPreference() {
        val biometricManager = BiometricManager.from(requireContext())
        val deviceCapable = biometricManager.isHardwareAvailable()
        val userHasEnabledCapability = biometricManager.isAuthenticatorAvailable()

        requirePreference<SwitchPreference>(R.string.pref_key_private_browsing_locked_enabled).apply {
            title = getString(R.string.preferences_tor_lock_screen_title, getString(R.string.app_name))
            summary = getString(R.string.preferences_tor_lock_screen_summary, getString(R.string.app_name))
            isChecked = context.settings().privateBrowsingLockedEnabled &&
                    biometricManager.isAuthenticatorAvailable()
            isVisible = deviceCapable
            isEnabled = userHasEnabledCapability

            setOnPreferenceChangeListener { preference, newValue ->
                val pbmLockEnabled = newValue as? Boolean
                    ?: return@setOnPreferenceChangeListener false

                val titleRes = if (pbmLockEnabled) {
                    R.string.tor_authentication_enable_lock
                } else {
                    R.string.tor_authentication_disable_lock
                }

                DefaultBiometricUtils.bindBiometricsCredentialsPromptOrShowWarning(
                    titleRes = titleRes,
                    titleRes2 = R.string.app_name,
                    view = requireView(),
                    onShowPinVerification = { intent -> startForResult.launch(intent) },
                    onAuthSuccess = {
                        onSuccessfulAuthenticationUsingPrimaryPrompt(
                            pbmLockEnabled = pbmLockEnabled,
                            preference = preference,
                        )
                    },
                    onAuthFailure = { },
                )

                // Cancel toggle change until biometric is successful
                false
            }
        }

        requirePreference<Preference>(R.string.pref_key_private_browsing_lock_device_feature_enabled).apply {
            title = getString(R.string.tor_authentication_lock_device_feature_disabled, getString(R.string.app_name))
            isVisible = deviceCapable && !userHasEnabledCapability

            setOnPreferenceClickListener {
                context.startActivity(Intent(Settings.ACTION_SECURITY_SETTINGS))
                true
            }
        }
    }

    private fun setupRadioGroups() {
        addToRadioGroup(
            listRadioButton,
            gridRadioButton,
        )

        addToRadioGroup(
            radioManual,
            radioOneDay,
            radioOneMonth,
            radioOneWeek,
        )
    }

    private fun sendTabViewTelemetry() {
        if (listRadioButton.isChecked && !gridRadioButton.isChecked) {
            Events.tabViewChanged.record(Events.TabViewChangedExtra("list"))
        } else {
            Events.tabViewChanged.record(Events.TabViewChangedExtra("grid"))
        }
    }

    private fun enableInactiveTabsSetting() {
        inactiveTabsCategory.apply {
            isEnabled = true
        }
    }

    private fun disableInactiveTabsSetting() {
        inactiveTabsCategory.apply {
            isEnabled = false
            inactiveTabs.isChecked = false
            context.settings().inactiveTabsAreEnabled = false
        }
    }
}
