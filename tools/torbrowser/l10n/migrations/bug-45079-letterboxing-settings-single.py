from fluent.migrate.helpers import transforms_from


def migrate(ctx):
    ctx.add_transforms(
        "base-browser.ftl",
        "base-browser.ftl",
        transforms_from(
            """
letterboxing-settings-group =
    .label = { COPY_PATTERN(path, "letterboxing-header") }
    .description = { COPY_PATTERN(path, "letterboxing-overview") }
""",
            path="base-browser.ftl",
        ),
    )
