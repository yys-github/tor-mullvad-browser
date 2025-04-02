# ⤵️ Rebase Stable

## **Bookkeeping**

- [ ] Link this issue to the appropriate [Release Prep](https://gitlab.torproject.org/tpo/applications/tor-browser-build/-/issues/?sort=updated_desc&state=opened&label_name%5B%5D=Apps%3A%3AType%3A%3AReleasePreparation) issue.

## **Rebase**

The step-by-step rebase process is detailed on the [Rebase Process](https://gitlab.torproject.org/tpo/applications/wiki/-/wikis/Development-Information/Rebase/Rebase-Process) wiki page. Refer to it for detailed instructions on how to perform each step.

- Rebase Tor Browser
  - Prepare the rebase
    - [ ] Get the Firefox tag
    - [ ] Freeze the current default branch
    - [ ] Create the target branch
  - Do the rebase
    - [ ] Cherry-pick commits until `tor-browser-...-build1`
    - [ ] Squash (`git rebase --autosquash FIREFOX_...-build1`)
    - [ ] Cherry-pick the remaining commits
    - [ ] Reorder commits
  - Merge
    - [ ] Perform a self-review  (`git range-diff` + diff of diffs)
    - [ ] Run linters
  - Tag
    - [ ] `tor-browser-...-build1`
    - [ ] `base-browser-...-build1`

---

/label ~"Apps::Product::TorBrowser"
/label ~"Apps::Type::Rebase"
/label ~"Apps::Impact::High"
/label ~"Priority::Blocker"
