from fluent.migrate.helpers import transforms_from


def migrate(ctx):
    # NOTE: preferences.ftl comes from firefox-l10n repository, under the path:
    #   <locale>/browser/browser/preferences/preferences.ftl
    # which will need to be added the translations worktree under the path:
    #   <locale>/preferences.ftl
    # prior to running this migration.
    ctx.add_transforms(
        "base-browser.ftl",
        "base-browser.ftl",
        transforms_from(
            """
browser-layout-show-sidebar-limited =
    .label = { COPY_PATTERN(preferences_path, "browser-layout-show-sidebar2.label") }
    .description = { COPY_PATTERN(path, "browser-layout-show-sidebar-desc-limited") }
""",
            path="base-browser.ftl",
            preferences_path="preferences.ftl",
        ),
    )
