import re

import fluent.syntax.ast as FTL
from fluent.migrate.helpers import transforms_from
from fluent.migrate.transforms import COPY_PATTERN, FluentSource
from fluent.syntax.visitor import Visitor


class RemoveAnchorVisitor(Visitor):
    """Class to remove <a> and </a> wrappers from a Fluent TextElement."""

    def __init__(self):
        # Good enough regex for our needs that will match starting and ending
        # tags.
        self._anchor_regex = re.compile(r"<\/?[aA](| [^>]*)>")
        super().__init__()

    def visit_TextElement(self, node):
        node.value = self._anchor_regex.sub("", node.value)


class RemoveAnchorTransform(FluentSource):
    """Class to remove <a> and </a> wrappers from a Fluent source."""

    def __call__(self, ctx):
        pattern = ctx.get_fluent_source_pattern(self.path, self.key).clone()
        # Visit every node in the pattern, replacing each TextElement's content.
        RemoveAnchorVisitor().visit(pattern)
        return pattern


def migrate(ctx):
    ctx.add_transforms(
        "tor-browser.ftl",
        "tor-browser.ftl",
        transforms_from(
            """
tor-bridges-group =
    .label = { COPY_PATTERN(path, "tor-bridges-heading") }
    .description = { COPY_PATTERN(path, "tor-bridges-overview") }
tor-bridges-add-bridges-group =
    .label = { COPY_PATTERN(path, "tor-bridges-add-bridges-heading") }
tor-bridges-replace-bridges-group =
    .label = { COPY_PATTERN(path, "tor-bridges-replace-bridges-heading") }
tor-bridges-choose-built-in-button =
    .label = { COPY_PATTERN(path, "tor-bridges-select-built-in-description") }
tor-bridges-enter-bridges-button =
    .label = { COPY_PATTERN(path, "tor-bridges-add-addresses-description") }
tor-bridges-find-more-group =
    .label = { COPY_PATTERN(path, "tor-bridges-find-more-heading") }
    .description = { COPY_PATTERN(path, "tor-bridges-find-more-description") }
tor-bridges-request-button2 =
    .label = { COPY_PATTERN(path, "tor-bridges-request-button") }
tor-bridges-source-email-link =
    .label = { COPY_PATTERN(path, "tor-bridges-provider-email-name") }
    .description = { COPY_PATTERN(path, "tor-bridges-provider-email-instruction") }
""",
            path="tor-browser.ftl",
        )
        + [
            FTL.Message(
                id=FTL.Identifier("tor-bridges-source-telegram-link"),
                value=None,
                attributes=[
                    FTL.Attribute(
                        id=FTL.Identifier("label"),
                        value=COPY_PATTERN(
                            "tor-browser.ftl",
                            "tor-bridges-provider-telegram-name",
                        ),
                    ),
                    FTL.Attribute(
                        id=FTL.Identifier("description"),
                        value=RemoveAnchorTransform(
                            "tor-browser.ftl",
                            "tor-bridges-provider-telegram-instruction",
                        ),
                    ),
                ],
            ),
            FTL.Message(
                id=FTL.Identifier("tor-bridges-source-web-link"),
                value=None,
                attributes=[
                    FTL.Attribute(
                        id=FTL.Identifier("label"),
                        value=COPY_PATTERN(
                            "tor-browser.ftl",
                            "tor-bridges-provider-web-name",
                        ),
                    ),
                    FTL.Attribute(
                        id=FTL.Identifier("description"),
                        value=RemoveAnchorTransform(
                            "tor-browser.ftl",
                            "tor-bridges-provider-web-instruction",
                        ),
                    ),
                ],
            ),
        ],
    )
