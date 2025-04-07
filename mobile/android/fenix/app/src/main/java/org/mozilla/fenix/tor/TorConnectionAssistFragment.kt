/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

package org.mozilla.fenix.tor

import android.graphics.Color
import android.graphics.Typeface
import android.os.Build
import android.os.Bundle
import android.text.SpannableString
import android.text.Spanned
import android.text.TextPaint
import android.text.method.LinkMovementMethod
import android.text.style.ClickableSpan
import android.util.Log
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import androidx.appcompat.content.res.AppCompatResources
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.DividerDefaults
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.adaptive.ExperimentalMaterial3AdaptiveApi
import androidx.compose.material3.adaptive.currentWindowDpSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.ViewCompositionStrategy
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.view.isEmpty
import androidx.fragment.app.Fragment
import androidx.fragment.app.activityViewModels
import androidx.fragment.app.viewModels
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.repeatOnLifecycle
import androidx.navigation.fragment.findNavController
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.launch
import mozilla.components.support.base.feature.UserInteractionHandler
import mozilla.components.ui.colors.PhotonColors
import mozilla.components.ui.colors.R as colorsR
import org.mozilla.fenix.HomeActivity
import org.mozilla.fenix.R
import org.mozilla.fenix.databinding.FragmentTorConnectionAssistBinding
import org.mozilla.fenix.e2e.SystemInsetsPaddedFragment
import org.mozilla.fenix.ext.hideToolbar

class TorConnectionAssistFragment : Fragment(), UserInteractionHandler, SystemInsetsPaddedFragment {

    private val TAG = "TorConnectionAssistFrag"
    private val progressViewModel: TorBootstrapProgressViewModel by viewModels()
    private val quickstartViewModel: QuickstartViewModel by activityViewModels()
    private val torConnectionAssistViewModel : TorConnectionAssistViewModel by viewModels()

    private var _binding: FragmentTorConnectionAssistBinding? = null
    private val binding get() = _binding!!

    private val firstMenuItem: MutableStateFlow<String> by lazy {
        MutableStateFlow(getString(R.string.connection_assist_automatic_country_detection))
    }

    override fun onCreateView(
        inflater: LayoutInflater,
        container: ViewGroup?,
        savedInstanceState: Bundle?,
    ): View {
        _binding = FragmentTorConnectionAssistBinding.inflate(
            inflater, container, false,
        )

        viewLifecycleOwner.lifecycleScope.launch {
            repeatOnLifecycle(Lifecycle.State.STARTED) {
                torConnectionAssistViewModel.collectTorConnectStage()
            }
        }

        torConnectionAssistViewModel.shouldOpenHome.observe(viewLifecycleOwner) {
            Log.d(TAG, "shouldOpenHome = $it")
            if (it) {
                openHome()
            }
        }

        return binding.root
    }

    override fun onResume() {
        super.onResume()
        hideToolbar()
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, savedInstanceState)

        binding.torBootstrapButton1.setTypeface(null, Typeface.BOLD)
        binding.torBootstrapButton2.setTypeface(null, Typeface.BOLD)

        viewLifecycleOwner.lifecycleScope.launch {
            repeatOnLifecycle(Lifecycle.State.STARTED) {
                torConnectionAssistViewModel.torConnectScreen.collect { screen ->
                    Log.d(TAG, "torConnectScreen is $screen")
                    showScreen(screen)
                }
            }
        }

        quickstartViewModel.quickstart().observe(
            viewLifecycleOwner,
        ) {
            binding.quickstartSwitch.isChecked = it
        }

        progressViewModel.progress.observe(
            viewLifecycleOwner,
        ) { progress ->
            setProgressBarCompat(progress)
        }

    }

    private fun setProgressBarCompat(progress: Int) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
            binding.torBootstrapProgressBar.setProgress(progress, true)
        } else {
            binding.torBootstrapProgressBar.progress = progress
        }
    }

    private fun showScreen(screen: ConnectAssistUiState) {
        setProgressBar(screen)
        setSettingsButton(screen)
        setBackButton(screen)
        setTorConnectImage(screen)
        setTitle(screen)
        setQuickStart(screen)
        updateRegionDropdown(screen)
        setButton1(screen)
        setButton2(screen)
        setSplashLogo(screen)
    }

    private fun setProgressBar(screen: ConnectAssistUiState) {
        binding.torBootstrapProgressBar.visibility =
            if (screen.progressBarVisible) View.VISIBLE else View.GONE
        binding.torBootstrapProgressBar.progressBackgroundTintList = AppCompatResources.getColorStateList(
            requireContext(),
            screen.progressBackgroundTintColorResource,
        )
    }

    private fun setSettingsButton(screen: ConnectAssistUiState) {
        binding.settingsButtonImage.imageTintList = AppCompatResources.getColorStateList(
            requireContext(),
            R.color.settings_button_white,
        )
        binding.settingsButton.visibility = if (screen.settingsButtonVisible) View.VISIBLE else View.GONE
        binding.settingsButton.setOnClickListener {
            openSettings()
        }
    }

    private fun setBackButton(screen: ConnectAssistUiState) {
        binding.backButtonImage.imageTintList = AppCompatResources.getColorStateList(
            requireContext(),
            R.color.settings_button_white,
        )
        binding.backButton.visibility = if (screen.backButtonVisible) View.VISIBLE else View.INVISIBLE
        binding.backButton.setOnClickListener {
            onBackPressed()
        }
    }

    private fun setTorConnectImage(screen: ConnectAssistUiState) {
        binding.torConnectImage.visibility = if (screen.torConnectImageVisible) View.VISIBLE else View.GONE
        binding.torConnectImage.setImageResource(screen.torConnectImageResource)
    }

    private fun setTitle(screen: ConnectAssistUiState) {
        binding.titleLargeTextView.visibility =
            if (screen.titleLargeTextViewVisible) View.VISIBLE else View.GONE
        binding.titleLargeTextView.text = getString(screen.titleLargeTextViewTextStringResource)
        binding.titleDescription.visibility =
            if (screen.titleDescriptionVisible) View.VISIBLE else View.GONE
        if (screen.learnMoreStringResource != null && screen.internetErrorDescription != null) {
            val learnMore: String = "" // getString(screen.learnMoreStringResource) tor-browser#43198 uncomment and add back once we have the "Learn more" screens for relevant pages
            val internetErrorDescription: String =
                if (screen.internetErrorDescription1 == null) {
                    getString(
                        screen.internetErrorDescription,
                        learnMore,
                    )
                } else if (screen.internetErrorDescription2 == null) {
                    getString(
                        screen.internetErrorDescription,
                        getString(screen.internetErrorDescription1),
                        learnMore,
                    )
                } else {
                    getString(
                        screen.internetErrorDescription,
                        getString(screen.internetErrorDescription1),
                        getString(screen.internetErrorDescription2),
                        learnMore,
                    )
                }
            handleDescriptionWithClickable(internetErrorDescription, learnMore)
        } else if (screen.titleDescriptionTextStringResource != null) {
            binding.titleDescription.text = getString(screen.titleDescriptionTextStringResource)
        }
    }

    private fun setQuickStart(screen: ConnectAssistUiState) {
        binding.quickstartSwitch.visibility =
            if (screen.quickstartSwitchVisible) View.VISIBLE else View.GONE
        binding.quickstartSwitch.setOnCheckedChangeListener { _, isChecked ->
            quickstartViewModel.quickstartSet(isChecked)
        }
    }

    private fun updateRegionDropdown(screen: ConnectAssistUiState) {
        if (screen.regionDropDownVisible) {
            firstMenuItem.value = getString(screen.regionDropDownDefaultItem)
            if (binding.regionDropDown.isEmpty()) {
                binding.regionDropDown.apply {
                    setContent {
                        setViewCompositionStrategy(ViewCompositionStrategy.DisposeOnViewTreeLifecycleDestroyed)
                        RegionDropDown()
                    }
                }
                torConnectionAssistViewModel.fetchFrequentRegions()
                torConnectionAssistViewModel.fetchRegionNames()
            }

            if (screen == ConnectAssistUiState.ChooseRegion || screen == ConnectAssistUiState.ConfirmRegion || screen == ConnectAssistUiState.RegionNotFound) {
                torConnectionAssistViewModel.selectDefaultRegion()
            }

            binding.regionDropDown.visibility = View.VISIBLE
        } else {
            binding.regionDropDown.visibility = View.GONE
        }
    }

    @Composable
    fun RegionDropDown(
        textStyle: TextStyle = TextStyle(
            fontSize = 16.sp,
            lineHeight = 24.sp,
            fontWeight = FontWeight(400),
            color = PhotonColors.LightGrey05,
            letterSpacing = 0.15.sp,
        ),
        labelTextStyle: TextStyle = TextStyle(
            fontSize = 14.sp,
            lineHeight = 20.sp,
            fontWeight = FontWeight(400),
            color = PhotonColors.LightGrey40,
            letterSpacing = 0.25.sp,
        ),
    ) {
        var expanded by rememberSaveable { mutableStateOf(false) }
        Box(
            modifier = Modifier.fillMaxWidth()
        ) {
            TextButton(
                onClick = { expanded = !expanded },
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp),
            ) {
                Column {
                    Text(
                        getString(R.string.connection_assist_unblock_the_internet_in_country_or_region),
                        color = PhotonColors.LightGrey05,
                        modifier = Modifier.padding(bottom = 8.dp),
                        fontSize = 14.sp,
                    )
                    Row {
                        Text(
                            torConnectionAssistViewModel.regionCodeNameMap.collectAsState().value?.get(
                                torConnectionAssistViewModel.selectedCountryCode.collectAsState().value,
                            ) ?: firstMenuItem.collectAsState().value,
                            color = PhotonColors.LightGrey05,
                            fontSize = 14.sp,
                        )
                        Spacer(modifier = Modifier.weight(1f))
                        Image(
                            painterResource(mozilla.components.ui.icons.R.drawable.unthemed_dropdown_arrow),
                            contentDescription = null,
                        )
                    }
                    HorizontalDivider(Modifier, thickness = 1.dp, color = PhotonColors.LightGrey05)
                }
            }
            DropdownMenu(
                expanded = expanded,
                onDismissRequest = { expanded = false },
                offset = DpOffset(x = 24.dp, y = 0.dp),
                modifier = Modifier.background(color = PhotonColors.Violet90)
            ) {
                @OptIn(ExperimentalMaterial3AdaptiveApi::class)
                LazyColumn(
                        Modifier
                            .width(250.dp)
                            .height(currentWindowDpSize().height) // fixMaxHeight() doesn't work, use this instead
                    ) {
                        item {
                            RegionDropdownMenuItem(
                                "automatic",
                                firstMenuItem.collectAsState().value,
                                dismissAction = { expanded = false },
                                textStyle = textStyle,
                            )
                        }
                        if (torConnectionAssistViewModel.frequentRegionCodes.value?.isEmpty() == false) {
                            item {
                                HorizontalDivider(Modifier, DividerDefaults.Thickness, color = PhotonColors.Ink05)
                                DropdownMenuItem(
                                    text = {
                                        Text(
                                            text = getString(R.string.connection_assist_frequently_selected_locations),
                                            style = labelTextStyle,
                                        )
                                    },
                                    onClick = {},
                                )
                            }
                            viewLifecycleOwner.lifecycleScope.launch {
                                repeatOnLifecycle(Lifecycle.State.STARTED) {
                                    torConnectionAssistViewModel.frequentRegionCodes.collect { codes ->
                                        if (codes != null) {
                                            for (code in codes) {
                                                item {
                                                    RegionDropdownMenuItem(
                                                        code,
                                                        torConnectionAssistViewModel.regionCodeNameMap.collectAsState().value?.get(
                                                            code,
                                                        ),
                                                        dismissAction = { expanded = false },
                                                        textStyle = textStyle,
                                                    )
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            item {
                                HorizontalDivider(Modifier, DividerDefaults.Thickness, color = PhotonColors.Ink05)
                                DropdownMenuItem(
                                    text = {
                                        Text(
                                            text = getString(R.string.connection_assist_other_locations),
                                            style = labelTextStyle,
                                        )
                                    },
                                    onClick = {}
                                )
                            }
                        }
                        viewLifecycleOwner.lifecycleScope.launch {
                            repeatOnLifecycle(Lifecycle.State.STARTED) {
                                torConnectionAssistViewModel.regionCodeNameMap.collect { regions ->
                                    if (regions != null) {
                                        for (region in regions.toList()) {
                                            item {
                                                RegionDropdownMenuItem(
                                                    region.first, region.second,
                                                    dismissAction = { expanded = false },
                                                    textStyle = textStyle,
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
            }
        }
    }

    @Composable
    fun RegionDropdownMenuItem(
        key: String,
        text: String?,
        dismissAction: () -> Unit,
        textStyle: TextStyle,
    ) {
        DropdownMenuItem(
            modifier = Modifier.padding(start = 8.dp),
            text = { Text(
                text ?: return@DropdownMenuItem,
                style = textStyle,
            ) },
            onClick = {
                torConnectionAssistViewModel.selectedCountryCode.value = key
                updateButton1(torConnectionAssistViewModel.torConnectScreen.value)
                dismissAction()
            },
        )
    }

    private fun setButton1(screen: ConnectAssistUiState) {
        binding.torBootstrapButton1.apply {
            visibility =
                if (screen.torBootstrapButton1Visible) View.VISIBLE else View.GONE
            text = getString(screen.torBootstrapButton1TextStringResource)
            setOnClickListener {
                if (screen.torBootstrapButton1ShouldOpenSettings) {
                    openTorConnectionSettings()
                } else {
                    torConnectionAssistViewModel.handleConnect(screen)
                }
            }
            updateButton1(screen)
        }
    }

    private fun updateButton1(screen: ConnectAssistUiState) {
        binding.torBootstrapButton1.apply {
            if (!torConnectionAssistViewModel.button1ShouldBeDisabled(screen)) {
                isEnabled = true
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
                isEnabled = false
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

    private fun setButton2(screen: ConnectAssistUiState) {
        binding.torBootstrapButton2.backgroundTintList = AppCompatResources.getColorStateList(
            requireContext(),
            R.color.configure_connection_button_white,
        )
        binding.torBootstrapButton2.visibility =
            if (screen.torBootstrapButton2Visible) View.VISIBLE else View.GONE
        if (screen.torBootstrapButton2ShouldRestartApp) {
            binding.torBootstrapButton2.text =
                screen.torBootstrapButton2TextStringResource?.let {
                    getString(
                        it,
                        getString(R.string.app_name),
                    )
                }
        } else {
            binding.torBootstrapButton2.text =
                screen.torBootstrapButton2TextStringResource?.let {
                    getString(
                        it,
                    )
                }
        }
        binding.torBootstrapButton2.setOnClickListener {
            if (screen.torBootstrapButton2ShouldOpenSettings) {
                openTorConnectionSettings()
            } else if (screen.torBootstrapButton2ShouldRestartApp) {
                (requireActivity() as HomeActivity).restartApplication()
            } else {
                torConnectionAssistViewModel.cancelTorBootstrap()
            }
        }
    }

    private fun setSplashLogo(screen: ConnectAssistUiState) {
        binding.wordmarkLogo.visibility = if (screen.wordmarkLogoVisible) View.VISIBLE else View.GONE
    }

    /**
     * from https://stackoverflow.com/questions/10696986/how-to-set-the-part-of-the-text-view-is-clickable
     */
    private fun handleDescriptionWithClickable(errorDescription: String, learnMore: String) {
        val errorDescriptionSpannableString = SpannableString(errorDescription)
        val clickableSpan: ClickableSpan = object : ClickableSpan() {
            override fun onClick(textView: View) {
                showLearnMore()
            }

            override fun updateDrawState(drawState: TextPaint) {
                super.updateDrawState(drawState)
                drawState.isUnderlineText = true
            }
        }
        errorDescriptionSpannableString.setSpan(
            clickableSpan,
            errorDescription.length - learnMore.length,
            errorDescription.length,
            Spanned.SPAN_EXCLUSIVE_EXCLUSIVE,
        )
        binding.titleDescription.text = errorDescriptionSpannableString
        binding.titleDescription.movementMethod = LinkMovementMethod.getInstance()
        binding.titleDescription.highlightColor = Color.TRANSPARENT
    }

    private fun showLearnMore() {
        Log.d(TAG, "showLearnMore() tapped")
        //TODO("Not yet implemented")
    }

    private fun openHome() {
        Log.d(TAG, "openHome()")
        findNavController().navigate(
            TorConnectionAssistFragmentDirections.actionHome(),
        )
    }

    private fun openSettings(preferenceToScrollTo: String? = null) {
        findNavController().navigate(
            TorConnectionAssistFragmentDirections.actionTorConnectionAssistFragmentToSettingsFragment(
                preferenceToScrollTo,
            ),
        )
    }

    private fun openTorConnectionSettings() {
        openSettings(requireContext().getString(R.string.pref_key_connection))
    }

    override fun onBackPressed(): Boolean {
        torConnectionAssistViewModel.handleBackButtonPressed(requireActivity() as HomeActivity)
        return true
    }

}
