# 📋 Firefox Release Review

**NOTE** This issue presumes the branches and tags for the next Firefox release have already been created in tor-browser.git

## Bugzilla Triage

- [ ] Generate Bugzilla triage CSV
  - Run (from `tor-browser-build` root):
  ```bash
    ./tools/browser/generate-bugzilla-triage-csv ${FIREFOX_VERSION} ${TRIAGE_ISSUE_NUMBER} > out.csv
  ```
  - `${FIREFOX_VERSION}`: the major Firefox version of the nightly to review
    - **Example**: 129
  - `${TRIAGE_ISSUE_NUMBER}`: this `tor-browser` issue
    - **Example**: `43303`
  - **Example**:
    ```bash
    ./tools/browser/generate-bugzilla-triage-csv 129 43303 > 129.csv
    ```
- [ ] Attach the generated CSV file to the triage isssue
- [ ] Import to Google Sheets ( https://sheets.google.com )
  - Create blank spreadsheet
    - **Title**: `Bugzilla Triage ${VERSION}`
  - Import CSV: File > Import > Upload
    - **Import location**: "Replace spreadsheet"
    - **Separator type**: "Comma"
    - **Convert text to numbers, dates, and fomulas**: "✅"
  - Convert 'Review' column's issue cells to check-boxes:
    - Select relevant cells (i.e.: `A2:A1554` for in the 129 triage)
    - Insert > Checkbox
  - Convert 'Triaged by' cells to check-boxes
  - Share Spreadsheet
    - 🔒 Share > General access
      - Change `Restricted` to `Anyone with the link`
    - Post link in an internal note on this issue
- [ ] Assign requested reviewers to this issue
  - **NOTE**: We currently have 3 blocks of reviewers in rotation:
    - bea, boklm, ma1
    - dan_b, henry-x, pierov
    - clairehurst, morgan, jwilde
- [ ] Set the issue's `Due Date` to 10 weeks after this version's "Beta starts" date or 2 weeks after the next ESR's "Beta starts" date (whichever is sooner)
  - **Release Calendar**: https://whattrainisitnow.com/calendar/

## Release Notes Review

<!--
  Ticket author! Find and post links to the release notes here!
    - Release notes for users: https://www.firefox.com/en-US/releases/
    - Release notes for developers: https://developer.mozilla.org/en-US/docs/Mozilla/Firefox/Releases
-->

- [ ] Release Notes for Users (often includes multiple point releases):
  - https://www.firefox.com/firefox/${FIREFOX_VERSION}/releasenotes/
- [ ] Release Notes for Developers
  - https://developer.mozilla.org/en-US/docs/Mozilla/Firefox/Releases/${FIREFOX_VERSION}

---

- Bugzilla Triage and Release Notes Review Completed by:
  - [ ] reviewer 1 <!-- replace with reviewer name :) -->
  - [ ] reviewer 2 <!-- replace with reviewer name :) -->
  - [ ] reviewer 3 <!-- replace with reviewer name :) -->

/label ~"Apps::Product::TorBrowser"
/label ~"Apps::Type::Triage"
/label ~"Priority::Blocker"
/milestone %"Tor Browser 16.0"
