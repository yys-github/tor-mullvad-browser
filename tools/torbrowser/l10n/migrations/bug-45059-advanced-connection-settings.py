from fluent.migrate.helpers import transforms_from


def migrate(ctx):
    ctx.add_transforms(
        "tor-browser.ftl",
        "tor-browser.ftl",
        transforms_from(
            """
tor-advanced-group =
    .label = { COPY_PATTERN(path, "tor-advanced-settings-heading") }
""",
            path="tor-browser.ftl",
        ),
    )
