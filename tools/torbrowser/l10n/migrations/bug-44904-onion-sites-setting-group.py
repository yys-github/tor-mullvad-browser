from fluent.migrate.helpers import transforms_from


def migrate(ctx):
    ctx.add_transforms(
        "tor-browser.ftl",
        "tor-browser.ftl",
        transforms_from(
            """
onion-site-authentication-group =
    .label = { COPY_PATTERN(path, "onion-site-authentication-preferences-heading") }
    .description = { COPY_PATTERN(path, "onion-site-authentication-preferences-overview") }
onion-site-authentication-saved-keys-button =
    .label = { COPY_PATTERN(path, "onion-site-authentication-preferences-saved-keys-button") }
""",
            path="tor-browser.ftl",
        ),
    )
