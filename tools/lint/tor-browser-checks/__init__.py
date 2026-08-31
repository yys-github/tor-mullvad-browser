# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

import xml.etree.ElementTree as ET

from mozlint import result

ANDROID_NS = "{http://schemas.android.com/apk/res/android}"

# Component types that can carry android:exported.
EXPORTABLE_TAGS = ("activity", "activity-alias", "service", "receiver", "provider")

MAIN_ACTION = "android.intent.action.MAIN"
LAUNCHER_CATEGORY = "android.intent.category.LAUNCHER"


def _find_lineno(lines, name):
    # ElementTree doesn't track source positions. This is a best-effort fallback
    # instead: find the line where this element's own android:name is written
    # out. Good enough to jump to the right spot, but not a guarantee if `name`
    # shows up as some other attribute's value too.
    needle = f'name="{name}"'
    for lineno, line in enumerate(lines, start=1):
        if needle in line:
            return lineno
    return 0


def _has_launcher_intent_filter(el):
    # A MAIN+LAUNCHER intent-filter is the one case Android itself makes
    # exported="true" load-bearing: that's what lets the home screen (a
    # separate app, from the OS's point of view) resolve and start this
    # component when its icon is tapped. Nothing else needs to be exported
    # for the app to function -- everything else is a deliberate choice to
    # let other apps reach in, and should default to false.
    for intent_filter in el.findall("intent-filter"):
        actions = {a.get(ANDROID_NS + "name") for a in intent_filter.findall("action")}
        categories = {
            c.get(ANDROID_NS + "name") for c in intent_filter.findall("category")
        }
        if MAIN_ACTION in actions and LAUNCHER_CATEGORY in categories:
            return True
    return False


def _check_only_launcher_entries_are_exported(path, config):
    """Flags any exported component in a fenix AndroidManifest.xml that
    isn't a MAIN+LAUNCHER entry point.

    This enforces the rule directly: android:exported="true" is only ever
    structurally required for a component that's meant to be launched from
    the home screen. Anything else that's exported can be set to exported="false"
    instead, even if that means turning off the feature that needed it.

    Not autofixable: turning exported="false" off may mean losing the
    feature that needed it (e.g. PWA shortcut relaunching), which needs a
    human to decide whether that tradeoff is acceptable case by case.

    Only looks at AndroidManifest.xml fragments; anything else is skipped.
    """
    if not path.endswith("AndroidManifest.xml"):
        return []

    try:
        # Manifest fragments (e.g. src/nightly/AndroidManifest.xml) can be a
        # bare <manifest> with no <application> at all -- nothing to check.
        app = ET.parse(path).getroot().find("application")
    except ET.ParseError:
        return []
    if app is None:
        return []

    with open(path, encoding="utf-8") as f:
        lines = f.readlines()

    issues = []
    for tag in EXPORTABLE_TAGS:
        for el in app.findall(tag):
            if el.get(ANDROID_NS + "exported") != "true":
                continue
            if _has_launcher_intent_filter(el):
                continue

            name = el.get(ANDROID_NS + "name")

            issues.append(
                result.from_config(
                    config,
                    path=path,
                    lineno=_find_lineno(lines, name),
                    message=(
                        f'<{tag}> "{name}" is exported="true" but has no '
                        "MAIN+LAUNCHER intent-filter, so Android doesn't "
                        'require it to be exported -- set exported="false" '
                        "(see tor-browser#45145)."
                    ),
                    level="error",
                    rule="unnecessary-exported-component",
                ),
            )

    return issues


# Add new Tor Browser security checks here. Each one receives a file path
# and is responsible for deciding whether that path is relevant to it, so
# checks are free to target entirely different kinds of files.
#
# Note: Remember to extend the linter's `include`/`extensions` in
# tor-browser-checks.yml if a new check needs paths that aren't already covered.
CHECKS = [
    _check_only_launcher_entries_are_exported,
]


def lint(paths, config, **lintargs):
    results = []

    for path in paths:
        for check in CHECKS:
            results.extend(check(path, config))

    return results
