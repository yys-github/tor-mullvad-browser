/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

package org.mozilla.fenix.tor

import android.content.Context
import android.os.Bundle
import android.util.Log
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.Toast
import androidx.appcompat.content.res.AppCompatResources
import androidx.fragment.app.Fragment
import mozilla.components.ui.colors.R as colorsR
import org.mozilla.fenix.HomeActivity
import org.mozilla.fenix.R
import org.mozilla.fenix.ext.components
import org.mozilla.fenix.databinding.FragmentTorSecurityLevelPreferencesBinding
import androidx.core.content.edit
import org.mozilla.fenix.e2e.SystemInsetsPaddedFragment

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

        binding.description.text = getString(R.string.tor_security_level_warning, getString(R.string.app_name))

        updateSaveAndRestartButtonUI()

        val currentLevel: Int = requireContext().components.core.engine.settings.torSecurityLevel

        when (currentLevel) {
            TorSecurityLevel.STANDARD.level -> {
                binding.standardPreference.text =
                    getString(R.string.tor_security_level_standard_current_level)
                binding.securityLevelRadioGroup.check(binding.standardPreference.id)
            }

            TorSecurityLevel.SAFER.level -> {
                binding.saferPreference.text =
                    getString(R.string.tor_security_level_safer_current_level)
                binding.securityLevelRadioGroup.check(binding.saferPreference.id)
            }

            TorSecurityLevel.SAFEST.level -> {
                binding.safestPreference.text =
                    getString(R.string.tor_security_level_safest_current_level)
                binding.securityLevelRadioGroup.check(binding.safestPreference.id)
            }
        }

        binding.securityLevelRadioGroup.setOnCheckedChangeListener { _, checkedId ->
            binding.saveAndRestartButton.isEnabled = when (checkedId) {
                binding.standardPreference.id -> currentLevel != TorSecurityLevel.STANDARD.level
                binding.saferPreference.id -> currentLevel != TorSecurityLevel.SAFER.level
                binding.safestPreference.id -> currentLevel != TorSecurityLevel.SAFEST.level
                else -> throw Exception("unexpected checkedID of $checkedId")
            }

            updateSaveAndRestartButtonUI()
        }

        binding.saveAndRestartButton.setOnClickListener {

            Toast.makeText(
                requireContext(),
                R.string.tor_security_level_restarting,
                Toast.LENGTH_SHORT,
            ).show()

            val selectedSecurityLevel: Int =
                when (binding.securityLevelRadioGroup.checkedRadioButtonId) {
                    binding.standardPreference.id -> TorSecurityLevel.STANDARD.level
                    binding.saferPreference.id -> TorSecurityLevel.SAFER.level
                    binding.safestPreference.id -> TorSecurityLevel.SAFEST.level
                    else -> throw Exception("Unexpected checkedRadioButtonId of ${binding.securityLevelRadioGroup.checkedRadioButtonId}")
                }

            requireContext().components.core.geckoRuntime.settings.torSecurityLevel = selectedSecurityLevel

            requireActivity().getSharedPreferences("fenix_preferences", Context.MODE_PRIVATE).edit(
                commit = true,
            ) {
                putInt(
                    requireContext().getString(R.string.pref_key_tor_security_level),
                    selectedSecurityLevel,
                )
            }

            Thread.sleep(1000)

            (requireActivity() as HomeActivity).restartApplication()
        }

        binding.cancelButton.setOnClickListener {
            @Suppress("DEPRECATION")
            requireActivity().onBackPressed()
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
