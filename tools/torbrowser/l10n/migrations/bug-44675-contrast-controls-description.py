from fluent.migrate.helpers import transforms_from


def migrate(ctx):
    ctx.add_transforms(
        "base-browser.ftl",
        "base-browser.ftl",
        transforms_from(
            """
preferences-contrast-control-fixed-color2 =
     .label = { COPY_PATTERN(path, "preferences-contrast-control-fixed-color.label") }
     .accesskey = { COPY_PATTERN(path, "preferences-contrast-control-fixed-color.accesskey") }
     .description = { COPY_PATTERN(path, "preferences-contrast-control-fixed-color-description") }
""",
            path="base-browser.ftl",
        ),
    )
