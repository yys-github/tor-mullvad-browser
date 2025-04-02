# ⤵️ Rebase Alpha

## **Bookkeeping**

- [ ] Link this issue to the appropriate [Release Prep](https://gitlab.torproject.org/tpo/applications/tor-browser-build/-/issues/?sort=updated_desc&state=opened&label_name%5B%5D=Apps%3A%3AType%3A%3AReleasePreparation) issue.
- [ ] Create "Firefox Release Review" issue for this version
  - **NOTE**: We have issues open through Firefox 153 so this can be skipped until we get to Firefox 154

## **Rebase**

The step-by-step rebase process is detailed on the [Rebase Process](https://gitlab.torproject.org/tpo/applications/wiki/-/wikis/Development-Information/Rebase/Rebase-Process) wiki page. Refer to it for detailed instructions on how to perform each step.

- Rebase application-services
  - uniffi-rs
    - Prepare the rebase
      - [ ] Verify if application-services has updated it's uniffi-rs version else skip this step
      - [ ] Get the [upstream](https://github.com/mozilla/uniffi-rs) tag
      - [ ] Freeze the current default branch (on both tor-browser and mullvad-browser)
      - [ ] Create the target branch (`X.XX.X`)
    - [ ] Rebase
    - Merge
      - [ ] Perform a self-review
      - [ ] Build
      - [ ] File a merge request
    - Tag and update the repository
      - [ ] Tag `vX.XX.X`
      - [ ] Make `X.XX.X` the default branch
  - application-services
    - Prepare the rebase
      - [ ] Get the [upstream](github.com/mozilla/application-services) tag
      - [ ] Freeze the current default branch
      - [ ] Create the target branch (`XXX.X-TORBROWSER`)
    - Do the rebase
      - [ ] Cherry-pick commits
      - [ ] Squash _all_ `fixup!` commits
    - Merge
      - [ ] Perform a self-review
      - [ ] Build
      - [ ] File a merge request
    - [ ] Tag and update the repository
      - [ ] Tag `vXXX.X-TORBROWSER-build1`
      - [ ] Make `XXX.X-TORBROWSER` the default branch

- Rebase Tor Browser
  - Prepare the rebase
    - [ ] Get the [Firefox](https://github.com/mozilla-firefox/firefox) tag
  - Do the rebase [Part 1]
    - [ ] Create the target branch (`tor-browser-XXX.0a1-YY.0-1`)
    - [ ] Cherry-pick commits until `tor-browser-(XXX - 1).0a1-YY.0-2-build1`
    - Optional: If your first rebase, complex, or difficult, can do an MR here for feedback.
    - [ ] Freeze the current default branch
    - [ ] Cherry-pick remaining commits (rest of tor-browser-(XXX - 1)a1-YY.Y-2)
    - Merge
      - [ ] Perform a self-review (`git range-diff`)
      - [ ] Run linters
      - [ ] Build and test
        - [ ] Desktop
        - [ ] Android
      - [ ] File a merge request
    - Tag and update the repository
      - [ ] Tag `tor-browser-...-1-build1`
      - [ ] Tag `tor-browser-...-1-build2`
      - [ ] Make `tor-browser-...-1` the default branch and freeze it
  - Do the rebase [Part 2]
    - [ ] Create the target branch (`tor-browser-...-2`)
    - [ ] Cherry-pick commits until `tor-browser-XXX...-1-build1`
    - [ ] Squash (`git rebase --autosquash FIREFOX_...`)
    - [ ] Cherry-pick the remaining commits
    - [ ] Reorder commits
      - [ ] Move Mozilla "Bug ZZZZZZZZ" issues to the very start
      - [ ] Move any Base Browser "BB TTTTT" issues into the BB range
      - [ ] Move `--fixups` next to their parent (`git rebase -i --autosquash FIREFOX_...` then change all the `fixup` to `pick`)
        - Note: also `drop` any commits marked `!dropme`
    - Merge
      - [ ] Perform a self-review (`git range-diff` + diff of diffs)
      - [ ] Run linters
      - [ ] Build and test
        - [ ] Desktop
        - [ ] Android
      - [ ] File a merge request
    - Tag and update the repository
      - [ ] `tor-browser-...-2-build1`
      - [ ] `base-browser-...-2-build1`
      - [ ] Make `tor-browser-...-2` the default branch
    - [ ] Send notification email to application-team about the rebase completion and new open branch
---

/label ~"Apps::Product::TorBrowser"
/label ~"Apps::Type::Rebase"
/label ~"Apps::Impact::High"
/label ~"Priority::Blocker"
