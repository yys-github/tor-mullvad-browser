from fluent.migrate.helpers import transforms_from


def migrate(ctx):
    ctx.add_transforms(
        "base-browser.ftl",
        "base-browser.ftl",
        transforms_from(
            """
letterboxing-window-size-group =
    .label = { COPY_PATTERN(path, "letterboxing-window-size-header") }
letterboxing-alignment-group =
    .label = { COPY_PATTERN(path, "letterboxing-alignment-header") }
    .description = { COPY_PATTERN(path, "letterboxing-alignment-description") }
letterboxing-alignment-top-option =
    .label = { COPY_PATTERN(path, "letterboxing-alignment-top") }
letterboxing-alignment-middle-option =
    .label = { COPY_PATTERN(path, "letterboxing-alignment-middle") }
letterboxing-disabled-message =
    .message = { COPY_PATTERN(path, "letterboxing-disabled-description") }
""",
            path="base-browser.ftl",
        ),
    )
