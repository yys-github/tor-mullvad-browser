/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

package org.mozilla.fenix.tor

import android.content.Context
import android.graphics.Typeface
import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.Toast
import androidx.appcompat.content.res.AppCompatResources
import androidx.core.content.edit
import androidx.fragment.app.Fragment
import mozilla.components.browser.engine.gecko.GeckoEngine
import mozilla.components.ui.colors.R as colorsR
import org.mozilla.fenix.HomeActivity
import org.mozilla.fenix.R
import org.mozilla.fenix.ext.components
import org.mozilla.fenix.databinding.FragmentTorSecurityLevelPreferencesBinding
import org.mozilla.fenix.e2e.SystemInsetsPaddedFragment
import org.mozilla.fenix.ext.showToolbar

class TorSecurityLevelFragment : Fragment(), SystemInsetsPaddedFragment {
    private var _binding: FragmentTorSecurityLevelPreferencesBinding? = null
    private val binding get() = _binding!!

    private val tag = "TorSecurityLevelFrag"

    override fun onCreateView(
        inflater: LayoutInflater,
        container: ViewGroup?,
        savedInstanceState: Bundle?,
    ): View {
        _binding = FragmentTorSecurityLevelPreferencesBinding.inflate(
            inflater, container, false,
        )

        return binding.root
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, savedInstanceState)
        // This is needed for when this fragment is navigated to via actionGlobalTorSecurityLevelFragment()
        // It has no effect otherwise
        showToolbar(getString(R.string.preferences_tor_security_level_options))

        binding.description.text = getString(R.string.tor_security_level_warning, getString(R.string.app_name))

        binding.saveAndRestartButton.setTypeface(null, Typeface.BOLD)
        binding.cancelButton.setTypeface(null, Typeface.BOLD)

        updateSaveAndRestartButtonUI()

        val currentLevel: Int = requireContext().components.settings.torSecurityLevel

        binding.customPreference.visibility = View.GONE
        binding.customPreferenceDescription.visibility = View.GONE

        when (currentLevel) {
            TorSecurityLevel.standard.level -> {
                binding.standardPreference.text =
                    getString(R.string.tor_security_level_standard_current_level)
                binding.securityLevelRadioGroup.check(binding.standardPreference.id)
            }

            TorSecurityLevel.safer.level -> {
                binding.saferPreference.text =
                    getString(R.string.tor_security_level_safer_current_level)
                binding.securityLevelRadioGroup.check(binding.saferPreference.id)
            }

            TorSecurityLevel.safest.level -> {
                binding.safestPreference.text =
                    getString(R.string.tor_security_level_safest_current_level)
                binding.securityLevelRadioGroup.check(binding.safestPreference.id)
            }

            TorSecurityLevel.custom.level -> {
                binding.customPreference.visibility = View.VISIBLE
                binding.customPreferenceDescription.visibility = View.VISIBLE
                binding.customPreference.text = getString(R.string.tor_security_level_custom_current_level)
                binding.securityLevelRadioGroup.check(binding.customPreference.id)
            }
        }

        binding.securityLevelRadioGroup.setOnCheckedChangeListener { _, checkedId ->
            binding.saveAndRestartButton.isEnabled = when (checkedId) {
                binding.standardPreference.id -> currentLevel != TorSecurityLevel.standard.level
                binding.saferPreference.id    -> currentLevel != TorSecurityLevel.safer.level
                binding.safestPreference.id   -> currentLevel != TorSecurityLevel.safest.level
                binding.customPreference.id   -> false
                else -> throw Exception("unexpected checkedID of $checkedId")
            }

            updateSaveAndRestartButtonUI()
        }

        binding.saveAndRestartButton.backgroundTintList = AppCompatResources.getColorStateList(
            requireContext(),
            R.color.disabled_connect_button_purple,
        )

        binding.saveAndRestartButton.setOnClickListener {

            Toast.makeText(
                requireContext(),
                R.string.tor_security_level_restarting,
                Toast.LENGTH_SHORT,
            ).show()

            val selectedSecurityLevel: TorSecurityLevel =
                when (binding.securityLevelRadioGroup.checkedRadioButtonId) {
                    binding.standardPreference.id -> TorSecurityLevel.standard
                    binding.saferPreference.id    -> TorSecurityLevel.safer
                    binding.safestPreference.id   -> TorSecurityLevel.safest
                    binding.customPreference.id   -> throw Exception("Custom preference not allowed to be saved")
                    else -> throw Exception("Unexpected checkedRadioButtonId of ${binding.securityLevelRadioGroup.checkedRadioButtonId}")
                }

            requireActivity().getSharedPreferences("fenix_preferences", Context.MODE_PRIVATE).edit(
                commit = true,
            ) {
                putInt(
                    requireContext().getString(R.string.pref_key_tor_security_level),
                    selectedSecurityLevel.level,
                )
            }

            (requireContext().components.core.engine as GeckoEngine).getTorIntegrationController()
                .setSecurityLevelBeforeRestart(selectedSecurityLevel.name)

            Thread.sleep(1000)
            (requireActivity() as HomeActivity).restartApplication()

        }

        binding.cancelButton.backgroundTintList = AppCompatResources.getColorStateList(
            requireContext(),
            R.color.settings_button_white,
        )
        binding.cancelButton.setOnClickListener {
            requireActivity().onBackPressedDispatcher.onBackPressed()
        }
    }

    private fun updateSaveAndRestartButtonUI() {
        binding.saveAndRestartButton.apply {
            if (binding.saveAndRestartButton.isEnabled) {
                backgroundTintList = AppCompatResources.getColorStateList(
                    requireContext(),
                    R.color.connect_button_purple,
                )
                setTextColor(
                    AppCompatResources.getColorStateList(
                        requireContext(),
                        colorsR.color.photonLightGrey05,
                    ),
                )
            } else {
                backgroundTintList = AppCompatResources.getColorStateList(
                    requireContext(),
                    R.color.disabled_connect_button_purple,
                )
                setTextColor(
                    AppCompatResources.getColorStateList(
                        requireContext(),
                        R.color.disabled_text_gray_purple,
                    ),
                )
            }
        }
    }
}
