import mozunit

LINTER = "tor-browser-checks"


def test_flags_exported_non_launcher_components(lint, paths):
    results = lint(paths("bad/AndroidManifest.xml"))
    assert len(results) == 4

    flagged_names = {
        "org.mozilla.gecko.BrowserApp",
        "org.mozilla.gecko.PermissiveApp",
        "org.mozilla.gecko.LauncherCategoryOnlyApp",
        "com.example.evil.UnlistedReceiver",
    }
    for expected_name in flagged_names:
        assert any(expected_name in r.message for r in results)

    for r in results:
        assert r.rule == "unnecessary-exported-component"


def test_allows_launcher_entries_and_unexported_components(lint, paths):
    results = lint(paths("good/AndroidManifest.xml"))
    assert len(results) == 0


if __name__ == "__main__":
    mozunit.main()
