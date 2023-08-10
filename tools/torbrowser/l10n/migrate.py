"""
Migration script for running Tor Browser migration recipes.

Based on the Firefox migration script, adapted for Tor Browser. See
https://firefox-source-docs.mozilla.org/l10n/migrations/ for more context and
information on how to write the migration recipes.


## Concepts

+ String. Here, when we say "string", it refers to a piece of text that can be
  translated. In practice, you can think of it as a being defined by a triple:
  a file path relative to a locale's directory, an ID, and some content. For
  example `("./tor-browser.ftl", "my-id", "my-id =\\n .label=Content")`.
  When a string is translated the file path and ID stay the same, whilst the
  translatable part of the content can vary.
+ Reference files. These are the en-US source Fluent files that contain the
  *manual* string changes made by a developer. For most migrations, there is
  only one reference file.
+ Migration recipe. A collection of instructions that take some string inputs
  and transforms them into new strings. These are meant to mirror the manual
  changes that were made in the reference files, in a way that can be applied
  to any locale.
+ Translation repository. This refers to a local copy of the `translation`
  repository (https://gitlab.torproject.org/tpo/translation). This repository
  contains a different branch for different collections of strings. For
  Tor Browser we usually use `tor-browser`, `base-browser` and
  `fenix-torbrowserstringsxml`. Each branch will contain an `en-US` directory,
  which contains the "source" files for translators to translate. These source
  files are taken from the `tor-browser` repository. The branches also contain
  a directory for each locale that is to be translated, in a file structure that
  reflects the source `en-US` directory. This is where the migration script will
  write changes to the translated files.


## Usage

You will likely want to run this with `./mach python`, so that it uses the
`fluent.migrate` and `fluent.syntax` packages provided with the `tor-browser`
repository.

For example, you might run this as

```
    ./mach python migrate.py --translation-git /home/user/translation --locales 'es-ES sr' ./migrations/bug-xxxx-my-migration-recipe.py
```

Here `/home/user/translation` is a local translation repository with one of its
branches checked out (see below). This will run the migration recipe given in
`./migrations/bug-xxxx-my-migration-recipe.py` for the `es-ES` and `sr`
sub-directories within the `translation/` directory.

### Changing the reference location

You can optionally run the script with `--reference_directory` to read the
reference en-US files from somewhere other than the `translation` directory.
This can be useful  when your manual changes have yet to reach the translation
repository, so you want to point the script to read the reference from the
`tor-browser` working directory instead. E.g.

```
    --reference-directory /home/user/tor-browser/toolkit/locales/en-US/toolkit/global/
```


## Recipes

Migration recipes are meant to mirror some manual change that was made to the
en-US source strings, in a way that can be reproduced in another locale.

### Example 1

For this example, a developer wants to remove the string content from
`old-strings.properties`:

```
my.old.string=Some string content
my.old.title=This is the tooltip
```

And add them to `tor-browser.ftl` instead, as:

```
my-new-string = Some string content
    .title = This is the tooltip
```

For the en-US source strings, the developer simply deletes the two lines in the
`old-strings.properties` file (or the whole file, if these are the last two).
And they manually write in the content of `my-new-string` to `tor-browser.ftl`.
These manual changes will then eventually reach the `translation` repository to
be translated.

But to save translators making the same manual changes, or re-translating, the
developer can write a migration recipe that mirror's their manual changes. In
this case, this migration recipe would contain:

```
ctx.add_transforms(
    "tor-browser.ftl", # The path to write to.
    "tor-browser.ftl", # The path to the reference file.
    transforms_from(
        \"\"\"
my-new-string = { COPY(path, "my.old.string") }
    .title = { COPY(path, "my.old.title") }
\"\"\",
        path="old-strings.properties", # The path to read from.
    ),
)
```

Note, the first argument of `add_transforms` is the target path, which is
`tor-browser.ftl` in this case. This means that the migration script will try to
write the resulting `my-new-string` to the locale's `./tor-browser.ftl` file.
For example, to `translation/es-ES/tor-browser.ftl`.

The second argument refers to the *reference* path. In almost all cases the
reference file will have the same name and path as the target path, but it has a
different role. Here the path is relative to the reference directory, which
contains the en-US source files. In this case, by default this means that
`translation/en-US/tor-browser.ftl` will act as the reference, but the exact
path could be changed with the `--reference-directory` argument. The existing
`my-new-string` will be read from this file to confirm that the migration
recipe produces the correct result.

The file `old-strings.properties` is used as an input. This means that the
existing strings in the locale's `old-strings.properties` file will be read to
construct the new string `my-new-string`. For example, it will be read from
`translation/es-ES/old-strings.properties`.

The `COPY` acts like a Fluent function, which asks the transformation to copy
the content from the translated string directly.

### Example 2

For this example, a developer wants to swap a string in `tor-browser.ftl` from:

```
my-old-string2 = Some string content
    .title = This is the tooltip
```

to

```
my-new-string2 =
    .label = Some string content
    .title = This is the tooltip
```

The developer removes `my-old-string2` from the file and adds `my-new-string2`
instead. This manual change will then eventually reach the `translation`
repository for `my-new-string2` to be translated.

The developer also writes a migration recipe to mirror this change:

```
ctx.add_transforms(
    "tor-browser.ftl",
    "tor-browser.ftl",
    transforms_from(
        \"\"\"
my-new-string2 =
    .label = { COPY_PATTERN(path, "my-old-string2") }
    .title = { COPY_PATTERN(path, "my-old-string2.title") }
\"\"\",
        path="tor-browser.ftl",
    ),
)
```

Note that in this case the locale's `tor-browser.ftl` file is being used as both
the input and the output.

Note that for a Fluent-to-Fluent migration we use `COPY_PATTERN` rather than
`COPY`. The second argument provides an ID that either point's to the
Fluent Messages' value (roughly, the text content) by giving the Message's ID,
or one of its attributes by adding a corresponding suffix to the ID.

### Other examples

Migration recipes can support a lot more, including more advanced
transformations like removing or replacing HTML tags from content. There are
some existing recipes in the `./migrations` directory that can be used as a
reference.


## translation directory

In most cases, you will just need to check out a `translation` branch that
contains the translated strings used by the migration recipe, as well as the
Fluent file that will be written to. Usually, this will be a branch based on
the origin/upstream `tor-browser` or `base-browser` branches.

However, if the migration recipe requires source files that are in a different
branch to the Fluent file that is being written to, then you will need to
manually copy those files into the working directory before running the script.


## Script procedure

### Test en-US migration

The script will first run a test migration using the en-US strings. It will run
the migration using the files from `translation/en-US/` as the input. In the
previous example 1, this will read `my.old.string` and `my.old.title` from
`translation/en-US/old-strings.properties`. And in example 2, this will read
`my-old-string2` from `translation/en-US/tor-browser.ftl`.

If any of the input strings are missing, this indicates that there is likely a
mistake in the recipe or the translation directory and the script will be
aborted without running the migration for the other locales.

Otherwise, it then compares the result of this migration with the strings found
in the reference files. In the previous example 1, this will read the entry for
`my-new-string` from `<reference-dir>/tor-browser.ftl`, where `<reference-dir>`
is `translation/en-US/` unless `--reference-directory` is given to use a
different directory. If the reference entry for `my-new-string` differs from the
entry that the recipe automatically produced, then this indicates that there is
a mistake in the recipe and the script will be aborted.

### Locale migrations

If the test migration passed, then the recipe will be re-run for each of the
specified locales. If the locale is missing some strings or files that a single
transformation requires, then that transformation will be skipped and the
corresponding entry won't be written to the Fluent file. Otherwise the new
entries will be appended to the existing Fluent files.

#### Non-Fluent strings and Weblate queries

Some non-Fluent formats, like DTD and properties, will fill untranslated strings
with the source en-US content. And we don't want these untranslated strings to
be used for migrations. There is no metadata within the files to determine this,
and simply being the same as the en-US string is not a reliable indicator.

So for these strings the script will additionally make a remote query to the
Weblate API to fetch the metadata that indicates whether the string is
considered translated or not. If a transformation requires a string that is
marked as untranslated, it will be skipped.

NOTE: The Weblate metadata is not always reliable, so you should still manually
review the changes.

NOTE: When testing before, the Weblate API was limited to 100 requests per day
per IP address for unauthorised users. If this becomes an issue you may want to
run the migrations in limited batches of locales with new IP addresses. Or, you
can create a Weblate account and change this script to pass an authorisation
token with the request.
"""

import argparse
import atexit
import importlib
import json
import logging
import os
import sys
import urllib.request
from collections.abc import Iterator
from typing import Any, NamedTuple, Protocol, cast

from fluent.migrate.context import MigrationContext
from fluent.migrate.errors import EmptyLocalizationError, UnreadableReferenceError
from fluent.syntax import ast, parse, serialize


def in_red(string: str) -> str:
    """
    Wrap a string so that it is shown as red in the console.
    :param string: The string to wrap.
    :returns: The wrapped string.
    """
    return f"\x1b[1;91m{string}\x1b[0m"


def assert_keys(
    check_dict: dict[Any, Any], expected_keys: tuple[str, ...], url: str
) -> None:
    """
    Verify that the dictionary contains *at least* the expected keys,
    otherwise this throws.
    :param check_dict: The dictionary to check.
    :param url: The url where the dictionary was fetched from.
    """
    for key in expected_keys:
        if key not in check_dict:
            raise ValueError(f"Missing '{key}' in response for {url}")


class WeblateUnit:
    """
    Represents a Weblate "unit": roughly corresponds to a translated string.

    See https://docs.weblate.org/en/latest/api.html#units.
    """

    def __init__(self, data: dict[Any, Any], url: str) -> None:
        """
        :param data: The Weblate data response.
        :param url: The URL that the data came from.
        """
        assert_keys(data, ("context", "translated"), url)
        string_id = data["context"]
        translated = data["translated"]
        if not isinstance(string_id, str):
            raise ValueError(f"Unexpected 'context' type in response for {url}")
        if not isinstance(translated, bool):
            raise ValueError(f"Unexpected 'translated' type in response for {url}")

        self.string_id = string_id
        self.translated = translated


class WeblateTranslation:
    """
    Represents a Weblate "translation": roughly corresponds to the translation
    of a file for a given locale/language.

    See https://docs.weblate.org/en/latest/api.html#translations.
    """

    def __init__(self, data: dict[Any, Any], url: str) -> None:
        """
        :param data: The Weblate data response.
        :param url: The URL that the data came from.
        """
        assert_keys(data, ("filename", "units_list_url"), url)
        locale_path = data["filename"]
        units_url = data["units_list_url"]
        if not isinstance(locale_path, str):
            raise ValueError(f"Unexpected 'filename' type in response for {url}")
        if not isinstance(units_url, str):
            raise ValueError(f"Unexpected 'units_list_url' type in response for {url}")

        self.locale_path = locale_path
        self.units_url = units_url
        # The units, indexed by their string ID. Lazy-loaded and `None` whilst
        # unfilled.
        self.units: None | dict[str, WeblateUnit] = None


class WeblateComponent:
    """
    Represents a Weblate "component": roughly corresponds to a source (en-US)
    file that can be translated.

    See https://docs.weblate.org/en/latest/api.html#components.
    And https://docs.weblate.org/en/latest/user/basic.html.
    """

    def __init__(self, data: dict[Any, Any], url: str) -> None:
        """
        :param data: The Weblate data response.
        :param url: The URL that the data came from.
        """
        assert_keys(data, ("template", "translations_url", "branch"), url)
        template_path = data["template"]
        translations_url = data["translations_url"]
        branch = data["branch"]
        if not isinstance(template_path, str):
            raise ValueError(f"Unexpected 'template' type in response for {url}")
        if not isinstance(translations_url, str):
            raise ValueError(
                f"Unexpected 'translations_url' type in response for {url}"
            )
        if not isinstance(branch, str):
            raise ValueError(f"Unexpected 'branch' type in response for {url}")

        self.branch = branch
        # The path to the source (en-US) file.
        self.template_path = template_path
        # The URL to fetch the component's translations from.
        self.translations_url = translations_url
        # The translations data, indexed by the locale file paths. Lazy-loaded
        # and `None` whilst unfilled.
        self.translations: None | dict[str, WeblateTranslation] = None


class WeblateMetadata:
    """
    Manages and fetches string metadata from Weblate.
    """

    # The translation branches that we expect to be used for Weblate.
    _BROWSER_BRANCHES = ["tor-browser", "base-browser", "fenix-torbrowserstringsxml"]
    _WEBLATE_API_URL = "https://hosted.weblate.org/api/"
    _WEBLATE_API_TOR_COMPONENTS_URL = _WEBLATE_API_URL + "projects/tor/components/"

    def __init__(self) -> None:
        """
        Create a new instance for fetching Weblate metadata.
        """
        self.logger = logging.getLogger("weblate-metadata")
        self.logger.setLevel(logging.INFO)

        # The _components property will be lazy-loaded with Weblate data.
        self._components: dict[str, WeblateComponent] | None = None

    def _get_weblate_response(self, url: str) -> Any:
        """
        Make a single query to Weblate.
        :param url: The URL to fetch.
        :returns: The JSON response.
        """
        if not url.startswith(self._WEBLATE_API_URL):
            raise ValueError(f"Unexpected URL request: {url}")

        weblate_request = urllib.request.Request(
            url,
            # NOTE: can set the request header:
            # "Authorization": "Token WEBLATE_TOKEN"
            # to get around rate-limiting.
            # NOTE: We empty the User-Agent to override the default
            # value of "Python-urllib/{major}.{minor}".
            headers={"User-Agent": ""},
        )

        with urllib.request.urlopen(weblate_request) as response:
            return json.load(response)

    def _get_from_weblate(self, url: str) -> list[dict[Any, Any]]:
        """
        Fetch a list of results from Weblate.
        :param url: The initial URL to fetch a response from.
        :returns: The returned list of results.
        """
        ret: list[dict[Any, Any]] = []
        while url:
            response = self._get_weblate_response(url)
            if not isinstance(response, dict):
                raise ValueError(f"Unexpected response type for {url}")
            assert_keys(response, ("results", "next"), url)

            res_val = response["results"]
            if not isinstance(res_val, list):
                raise ValueError(f"Unexpected 'results' type for {url}")
            for res_item in res_val:
                if not isinstance(res_item, dict):
                    raise ValueError(f"Unexpected 'results' item type for {url}")
                ret.append(res_item)

            # Continue to fetch the next page, if it is present.
            # Expect the "next" url to be null
            # or the same url with "?page=2", etc.
            next_val = response["next"]
            if next_val is not None and not isinstance(next_val, str):
                raise ValueError(f"Unexpected 'next' type in response for {url}")
            url = "" if next_val is None else next_val
        return ret

    def _get_components(self) -> dict[str, WeblateComponent]:
        """
        Get all the Tor Browser Weblate components.

        We identify each Weblate component by its template path: the en-US
        file path in the translation repository.

        NOTE: We assume that the template path uniquely identifies the
        component, even though this is not guaranteed in general by Weblate.

        :returns: A dictionary of components, indexed by their en-US template
          path.
        """
        url = self._WEBLATE_API_TOR_COMPONENTS_URL
        if self._components is None:
            self._components = {
                comp.template_path: comp
                for comp in (
                    WeblateComponent(data, url) for data in self._get_from_weblate(url)
                )
                if comp.template_path and comp.branch in self._BROWSER_BRANCHES
            }
        return self._components

    def _get_translations(
        self, template_path: str
    ) -> dict[str, WeblateTranslation] | None:
        """
        Get the Weblate translations for a given Weblate component.

        We identify each translation by its file path: the locale-specific path
        that the translated file is written to.

        :param template_path: The component's en-US template path.
        :returns: A dictionary of translations, indexed by their locale file
          paths. Or `None` if no components were found.
        """
        component = self._get_components().get(template_path, None)
        if not component:
            self.logger.warning(f"No component in Weblate for {template_path}.")
            return None
        if component.translations is None:
            url = component.translations_url
            component.translations = {
                trans.locale_path: trans
                for trans in (
                    WeblateTranslation(data, url)
                    for data in self._get_from_weblate(url)
                )
            }
        return component.translations

    def _get_units(
        self, template_path: str, locale_path: str
    ) -> dict[str, WeblateUnit] | None:
        """
        Get the Weblate units for a given Weblate translation.

        We identify each unit by its string ID.

        :param template_path: The containing component's en-US template path.
        :param locale_path: The translation's locale file path.
        :returns: A dictionary of units, indexed by their string ID. Or `None`
          if no translations could be found.
        """
        translation_dict = self._get_translations(template_path)
        if translation_dict is None:
            return None
        translation = translation_dict.get(locale_path, None)
        if translation is None:
            self.logger.warning(f"No translation in Weblate for {locale_path}.")
            return None
        if translation.units is None:
            url = translation.units_url
            translation.units = {
                unit.string_id: unit
                for unit in (
                    WeblateUnit(data, url) for data in self._get_from_weblate(url)
                )
            }
        return translation.units

    def is_translated(self, file_path: str, locale: str, string_id: str) -> bool:
        """
        Test whether the given string is marked as translated on Weblate.

        :param file_path: The file to check the translation of, relative to the
          locale's directory.
        :param locale: The locale to check for.
        :param string_id: The ID of the string to check.
        """
        template_path = os.path.join("en-US", file_path)
        locale_path = os.path.join(locale, file_path)
        unit_dict = self._get_units(template_path, locale_path)
        if unit_dict is None:
            return False
        unit = unit_dict.get(string_id, None)
        if unit is None:
            self.logger.warning(f"No unit in Weblate for {locale_path}:{string_id}.")
            return False
        return unit.translated


class TorBrowserMigrationContext(MigrationContext):
    """
    Extension which adds some extra methods to use for tor-browser.
    """

    def __init__(self, locale: str, reference_dir: str, localization_dir: str) -> None:
        super().__init__(locale, reference_dir, localization_dir)

    def _fluent_keys(self, resource: ast.Resource) -> Iterator[str]:
        """
        Generate all the keys within a Fluent resource, where a key is an
        identifier for a Term, a Message, a Term attribute, or a Message
        attribute.
        :yield: The Fluent keys.
        """
        # ast.Resource, want to extract all Message and Term identifiers, as
        # well as their attributes.
        for entry in resource.body:
            if not isinstance(entry, (ast.Term, ast.Message)):
                continue
            key = entry.id.name
            if isinstance(entry, ast.Term):
                key = f"-{key}"
            if entry.value:
                yield key
            for attr in entry.attributes:
                yield f"{key}.{attr.id.name}"

    def tb_get_available_strings(self) -> set[tuple[str, str]]:
        """
        Get all the (path, string_id) pairs for all loaded localization
        resources.
        :returns: The string pairs.
        """
        all_strings: set[tuple[str, str]] = set()
        # ctx.localization_resources is a dict containing all the loaded
        # localization_resources that have been added during add_transforms.
        # { localization_path: resource }
        for path, resource in self.localization_resources.items():
            if path.endswith(".ftl"):
                all_strings.update((path, key) for key in self._fluent_keys(resource))
            else:
                # dictionary of { identifies: value } in legacy resource.
                all_strings.update((path, key) for key in resource.keys())
        return all_strings

    def tb_get_missing_resources(self) -> set[str]:
        """
        Get the missing localization resources.
        :returns: The set of resource paths.
        """
        return set(
            path
            for dep_set in self.dependencies.values()
            for path, string_id in dep_set
            if path not in self.localization_resources
        )

    def tb_get_transform(
        self, target_path: str, transform_id: str
    ) -> ast.Message | ast.Term | None:
        """
        Find a transformation node.
        :param target_path: The target path of the transformation.
        :param transform_id: The ID of the transformation, AKA the string ID.
        :returns: The matching transformation node, or `None` if there is no
          match.
        """
        for node in self.transforms[target_path]:
            if node.id.name == transform_id:
                return node
        return None

    def tb_get_reference_entry(
        self, target_path: str, transform_id: str
    ) -> ast.Message | ast.Term | None:
        """
        Find a reference Message or Term for a given transformation.
        :param target_path: The target path of the transformation.
        :param transform_id: The ID of the transformation, AKA the string ID.
        :returns: The matching reference entry for the transformation, or `None`
          if there is no match.
        """
        for entry in self.reference_resources[target_path].body:
            if (
                isinstance(entry, (ast.Term, ast.Message))
                and entry.id.name == transform_id
            ):
                return entry.clone()
        return None


class RecipeModule(Protocol):
    def migrate(self, context: MigrationContext) -> None: ...


class FluentSyntaxError(NamedTuple):
    path: str
    message: str
    line: int
    sample: str


class TorBrowserMigrator:
    """
    Performs a tor-browser migration.
    """

    def __init__(
        self,
        en_US_ref_dir: str,
        en_US_target_dir: str,
        locale_dirs: dict[str, str],
        recipe_module: RecipeModule,
        weblate_metadata: WeblateMetadata,
    ) -> None:
        """
        :param en_US_ref_dir: The path to the en-US directory where reference
          Fluent entries for the recipe can be found.
        :param en_US_target_dir: The path to the en-US directory where existing
          strings used by the recipe can be found.
        :param locale_dirs: The collection of locales to run the migrations for.
        :param recipe_module: The python module that contains the recipe
          instructions.
        :param weblate_metadata: The object to fetch Weblate metadata from.
        """
        self.logger = logging.getLogger("tor-browser-migrator")
        self.logger.setLevel(logging.INFO)
        self.en_US_ref_dir = en_US_ref_dir
        self.en_US_target_dir = en_US_target_dir
        self.locale_dirs = locale_dirs
        self.recipe_module = recipe_module
        self.weblate_metadata = weblate_metadata

    def run(self) -> None:
        """
        Run the migration.
        """
        if not self._check_en_US_resources():
            sys.exit(1)

        fluent_errors: list[FluentSyntaxError] = []
        for locale, locale_dir in self.locale_dirs.items():
            if locale == "en-US":
                print("", file=sys.stderr)
                self.logger.warning(
                    "Skipping running migration on 'en-US' files since this "
                    "locale should act as a reference only.\n"
                )
                continue
            fluent_errors.extend(self._run_locale(locale, locale_dir))

        if fluent_errors:
            print("\n", file=sys.stderr)
            self.logger.error(
                "Fluent parsing errors found for the following files. "
                "Migration does not need to be run again, but the following "
                "syntax errors should be fixed manually.\n"
                + "\n".join(
                    f"{in_red(err.path)}: line {err.line}: {err.message}: [[{err.sample}]]"
                    for err in fluent_errors
                )
            )

    def _run_locale(self, locale: str, locale_dir: str) -> list[FluentSyntaxError]:
        """
        Run a migration for a given locale.
        :param locale: The name of the locale.
        :param locale_dir: The path to the locale's directory.
        :returns: A list of Fluent errors in the written files.
        """
        print("\n\n", file=sys.stderr)
        self.logger.info(f"Migrating '{in_red(locale)}' locale\n")

        ctx = self._get_migration_context(locale, locale_dir)

        # NOTE: We do not use the existing ctx.serialize_changeset method.
        # The problem with this approach was that it would re-shuffle the order
        # of already existing strings to match the en-US locale.
        # But Weblate currently does not preserve the order of translated
        # strings: https://github.com/WeblateOrg/weblate/issues/11134
        # so this created extra noise in the diff.
        # Instead, we just always append transformations to the end of the
        # existing file.
        # Moreover, it would inject group comments into the translated files,
        # which Weblate does not handle well. Instead, we just do not add any
        # comments.
        #
        # In case we want to use it again in the future, here is a reference
        # to how it works:
        #
        # ctx.serialize_changeset expects a set of (path, identifier) of
        # localization resources that can be used to evaluate the
        # transformations.
        # e.g. ("example.dtd", "exampleStringInDTD")
        #      ("example.ftl", "some-message")
        #      ("example.ftl", "some-message.attribute")
        #
        # Mozilla splits its changesets into the authors/users who are
        # attributed to creating the identified sources, using hg blame and
        # checking the author to identify the user.
        # For tor-browser, we just want to apply all the changes possible in
        # one step, so we want to fill the changeset with all required
        # (path, identifier) pairs found in the localization resources.

        available_strings = ctx.tb_get_available_strings()
        wrote_file = False
        errors: list[FluentSyntaxError] = []

        for target_path, reference in ctx.reference_resources.items():
            translated_ids = [
                entry.id.name
                for entry in ctx.target_resources[target_path].body
                if isinstance(entry, (ast.Message, ast.Term))
                # NOTE: We're assuming that the Message and Term ids do not
                # conflict with each other.
            ]
            new_entries = []

            # Apply transfomations in the order they appear in the reference
            # (en-US) file.
            for entry in reference.body:
                if not isinstance(entry, (ast.Message, ast.Term)):
                    continue
                transform_id = entry.id.name
                transform = ctx.tb_get_transform(target_path, transform_id)
                if not transform:
                    # No transformation for this reference entry.
                    continue

                if transform_id in translated_ids:
                    self.logger.info(
                        f"Skipping transform {target_path}:{transform_id} "
                        f"for '{locale}' locale because it already has a "
                        f"translation."
                    )
                    continue

                # ctx.dependencies is a dict of dependencies for all
                # transformations
                # { (target_path, transform_identifier): set(
                #     (localization_path, string_identifier),
                # )}
                #
                # e.g. if we want to create a new fluent Message called
                # "new-string1", and it uses "oldString1" from "old-file1.dtd"
                # and "oldString2" from "old-file2.dtd". And "new-string2" using
                # "oldString3" from "old-file2.dtd", it would be
                # {
                #   ("new-file.ftl", "new-string1"): set(
                #     ("old-file1.dtd", "oldString1"),
                #     ("old-file2.dtd", "oldString2"),
                #   ),
                #   ("new-file.ftl", "new-string2"): set(
                #     ("old-file2.dtd", "oldString3"),
                #   ),
                # }
                dep_set = ctx.dependencies[(target_path, transform_id)]
                can_transform = True
                for dep in dep_set:
                    path, string_id = dep
                    if dep not in available_strings:
                        can_transform = False
                        self.logger.info(
                            f"Skipping transform {target_path}:{transform_id} "
                            f"for '{locale}' locale because it is missing the "
                            f"string {path}:{string_id}."
                        )
                        break
                    # Strings in legacy formats might have an entry in the file
                    # that is just a copy of the en-US strings.
                    # For these we want to check the Weblate metadata to ensure
                    # it is a translated string.
                    if not path.endswith(
                        ".ftl"
                    ) and not self.weblate_metadata.is_translated(
                        path,
                        locale,
                        string_id,
                    ):
                        can_transform = False
                        self.logger.info(
                            f"Skipping transform {target_path}:{transform_id} "
                            f"for '{locale}' locale because the string "
                            f"{path}:{string_id} has not been translated on "
                            "Weblate."
                        )
                        break
                if not can_transform:
                    continue

                # Run the transformation.
                new_entries.append(ctx.evaluate(transform))

            if not new_entries:
                continue

            full_path = os.path.join(locale_dir, target_path)
            print("", file=sys.stderr)
            self.logger.info(f"Writing to {full_path}")

            # For Fluent we can just serialize the transformations and append
            # them to the end of the existing file.
            resource = ast.Resource(new_entries)
            with open(full_path, "a") as file:
                file.write(serialize(resource))

            with open(full_path) as file:
                full_content = file.read()
            wrote_file = True
            # Collect any fluent parsing errors from the newly written file.
            errors.extend(
                FluentSyntaxError(
                    path=full_path, message=message, line=line, sample=sample
                )
                for message, line, sample in self._fluent_errors(full_content)
            )

        if not wrote_file:
            self.logger.info(f"No files written for '{locale}' locale.")
        return errors

    def _fluent_errors(self, fluent_content: str) -> Iterator[tuple[str, int, str]]:
        """
        Parse the given content as a Fluent file and generate all the syntax
        errors.
        :param fluent_content: The file content to parse.
        :yield: Each encountered Fluent syntax error as a triple of the message,
        the line number, and a sample.
        """
        resource = parse(fluent_content)
        for entry in resource.body:
            if not isinstance(entry, ast.Junk):
                continue
            for annotation in entry.annotations:
                start = 0
                if annotation.span:
                    start = annotation.span.start
                line = fluent_content[0:start].count("\n") + 1
                sample_start = max(start - 15, 0)
                sample = "…" + fluent_content[sample_start : sample_start + 30] + "…"
                yield annotation.message or "", line, sample

    def _get_migration_context(
        self, locale: str, locale_dir: str
    ) -> TorBrowserMigrationContext:
        """
        Create a new migration context.
        :param locale: The name of the locale.
        :param locale_dir: The directory path for the locale.
        :returns: The new context.
        """
        prev_missing_resources: set[str] = set()
        while True:
            ctx = TorBrowserMigrationContext(locale, self.en_US_ref_dir, locale_dir)

            try:
                self.recipe_module.migrate(ctx)
            except EmptyLocalizationError:
                # This case will be handled by missing_resources below.
                # NOTE: At the time of writing, add_transforms only throws if
                # ctx.localization_resources is empty after add_transforms,
                # which means whether it throws can depend on whether the
                # missing resource was found missing before or after some
                # non-missing resource was found. I.e. the order in which
                # add_transforms is called can influence whether add_transforms
                # will throw.
                # Therefore, we want to handle the case where it throws or does
                # not throw in the same way. We also need to create a new
                # context for the next run so that the early exit from
                # add_transforms in this run does not make a difference.
                pass

            missing_resources = ctx.tb_get_missing_resources()

            if not missing_resources:
                return ctx

            still_missing = missing_resources & prev_missing_resources
            if still_missing:
                # Unexpected to still be missing the same files after the
                # previous run.
                self.logger.error(
                    f"Still missing files in '{locale}' locale: "
                    + ", ".join(in_red(path) for path in still_missing)
                )
                sys.exit(1)

            for path in missing_resources:
                # Create an empty file to try and get migrate() to succeed
                # the next round.
                # NOTE: Missing strings within a resource is ok, we just want to
                # add the missing file to prevent add_transforms from throwing
                # to allow us to proceed.
                full_path = os.path.join(locale_dir, path)
                self.logger.info(f"Creating temporary empty file: {full_path}")
                # Throw if it already exists.
                file = open(full_path, "x")
                # Remove the empty file on exit if it is still empty.
                atexit.register(self._remove_if_empty, full_path)
                # Immediately close.
                file.close()

            # Try again with the newly added resources.
            # Don't expect it to throw EmptyLocalizationError the second time,
            # although it may still be missing resources if the last run threw
            # before localization_resources was fully populated.
            prev_missing_resources = missing_resources

    def _remove_if_empty(self, path: str) -> None:
        """
        Remove a file if it is empty.
        :param path: The path to remove.
        """
        if os.stat(path).st_size:
            self.logger.warning(f"{path} is no longer empty. Not deleting.")
            return
        os.remove(path)

    def _check_en_US_resources(self) -> bool:
        """
        Verify that the migration recipe works as expected using en-US strings.
        :returns: Whether the recipe worked as expected.
        """
        ctx = TorBrowserMigrationContext(
            "en-US", self.en_US_ref_dir, self.en_US_target_dir
        )

        have_error = False
        try:
            self.recipe_module.migrate(ctx)
        except EmptyLocalizationError:
            # Handle with localization_resources check.
            # NOTE: This throwing may have prevented further add_transforms from
            # proceeding, so we will only report missing string errors up to
            # this point.
            # Set have_error here just in case.
            have_error = True
        except UnreadableReferenceError:
            # Reference filename is printed before this.
            self.logger.error(
                "Missing an en-US reference file. Is --reference-directory correct?"
            )
            return False

        # Check each transform would create the same entry in the target as the
        # already existing reference file for en-US, using the existing en-US
        # localization files.
        # I.e. if we ran the transforms for en-US we expect to get the same file
        # as the reference.
        available_strings = ctx.tb_get_available_strings()
        for (target_path, transform_id), dep_set in ctx.dependencies.items():
            transform_name = in_red(f"{target_path}:{transform_id}")
            has_deps = True
            for dep in dep_set:
                path, string_id = dep
                if path not in ctx.localization_resources:
                    has_deps = False
                    self.logger.error(
                        f"Missing en-US localization file {in_red(path)} "
                        f" for transform {transform_name}"
                    )
                    continue
                if dep not in available_strings:
                    has_deps = False
                    self.logger.error(
                        "Missing en-US localization string "
                        + in_red(f"{path}:{string_id}")
                        + f" for transform {transform_name}"
                    )
            if not has_deps:
                have_error = True
                continue

            transformed = ctx.evaluate(ctx.tb_get_transform(target_path, transform_id))
            reference_entry = ctx.tb_get_reference_entry(target_path, transform_id)
            if reference_entry is None:
                self.logger.error(
                    f"Missing en-US reference entry for transform {transform_name}. "
                    "Is --reference-directory correct?"
                )
                have_error = True
                continue

            # Serialized the single transformed and reference and compare.
            transform_serialized = serialize(ast.Resource([transformed]))
            # Remove comment in reference for comparison.
            reference_entry.comment = None
            ref_serialized = serialize(ast.Resource([reference_entry]))
            if transform_serialized != ref_serialized:
                self.logger.error(
                    f"Transform {transform_name} would not produce the same "
                    "entry as the existing en-US reference when acting on "
                    "en-US localization files.\n"
                    f"Transform result:\n{transform_serialized}\n"
                    f"Reference entry:\n{ref_serialized}"
                )
                have_error = True

        return not have_error


def main(recipe_path: str, translation_dir: str, locales: str, ref_dir: str) -> None:
    """
    The main method.
    :param recipe_path: The path to the recipe script.
    :param translation_dir: The path to the translation repository directory.
    :param locales: The locales to run the recipe for, separated by spaces.
    :param ref_dir: The path to the reference directory.
    """
    if not os.path.isfile(recipe_path):
        print(f"{in_red(recipe_path)} is not a file.", file=sys.stderr)
        sys.exit(1)

    recipe_module_spec = importlib.util.spec_from_file_location("_recipe", recipe_path)
    recipe_module = cast(
        RecipeModule, importlib.util.module_from_spec(recipe_module_spec)
    )
    recipe_module_spec.loader.exec_module(recipe_module)

    def check_dir(path: str) -> str:
        if not os.path.isdir(path):
            print(f"{in_red(path)} is not a directory.", file=sys.stderr)
            sys.exit(1)
        return path

    translation_dir = check_dir(translation_dir)
    en_US_target_dir = check_dir(os.path.join(translation_dir, "en-US"))
    en_US_ref_dir = en_US_target_dir if ref_dir is None else check_dir(ref_dir)

    TorBrowserMigrator(
        en_US_ref_dir,
        en_US_target_dir,
        {
            locale: check_dir(os.path.join(translation_dir, locale))
            for locale in (l.strip() for l in locales.split(" "))
            if locale
        },
        recipe_module,
        WeblateMetadata(),
    ).run()


parser = argparse.ArgumentParser(
    description="Run a tor-browser localization migration. "
    "See documentation within migrate.py."
)
parser.add_argument(
    "--translation-git",
    required=True,
    metavar="<dir>",
    help="Location of the translation-git directory to read and write to.",
)
parser.add_argument(
    "--locales",
    required=True,
    metavar="'<locale1> <locale2> ...'",
    help="Set of locales to run the migration for, separated by space.",
)
parser.add_argument(
    "--reference-directory",
    metavar="<dir>",
    help="Location of the en-US directory that contains the reference changes "
    "that the recipe corresponds to. E.g. if the recipe constructs the string "
    "`my-new-string` in `./tor-browser.ftl`, it should point to a directory "
    "that contains the en-US `tor-browser.ftl` with `my-new-string` already "
    "within it. Defaults to the `en-US/` sub-directory within the "
    "translation-git directory.",
)
parser.add_argument(
    "recipe",
    help="The path to the migration recipe to run.",
)


parsed_args = parser.parse_args()

main(
    parsed_args.recipe,
    parsed_args.translation_git,
    parsed_args.locales,
    parsed_args.reference_directory,
)
