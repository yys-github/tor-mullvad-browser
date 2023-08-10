from fluent.migrate.helpers import transforms_from


def migrate(ctx):
    ctx.add_transforms(
        "tor-browser.ftl",
        "tor-browser.ftl",
        transforms_from(
            """
tor-connection-settings-pane =
    .heading = { COPY_PATTERN(path, "tor-connection-settings-heading") }
tor-connection-settings-nav-button = { COPY_PATTERN(path, "tor-connection-settings-heading") }
    .title = { COPY_PATTERN(path, "tor-connection-settings-heading") }
tor-connection-status-connect-button =
    .label = { COPY_PATTERN(path, "tor-connection-network-status-connect-button") }
""",
            path="tor-browser.ftl",
        ),
    )
