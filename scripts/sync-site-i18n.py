#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
"""Make the English text in the site's HTML the same text the English dictionary holds.

Every translatable element in `www/*.html` carries a `data-i18n` key and, inside it, the English
wording as the default a reader sees before the script runs. So one sentence lives in two places,
and they drifted: the dictionary said one thing and the markup another, both shipped, and which a
person read depended on whether JavaScript had finished.

The dictionary is the source. This copies its English values into the markup and reports any key the
markup uses that the dictionary does not define, which is the other way the two come apart.

Run it after editing `www/script.js`; `--check` fails instead of writing, for a gate.

An element whose content is not a single run of text — anything with a nested tag other than
`<strong>`, which several stack entries use as a label — is reported and left alone. Rewriting it
would flatten whatever is inside, and losing a link is worse than leaving one sentence to be fixed
by hand.
"""

import argparse
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "www" / "script.js"
PAGES = sorted((ROOT / "www").glob("*.html"))

OPENING = re.compile(
    r'<(?P<tag>[a-z0-9]+)(?P<attrs>[^>]*\bdata-i18n="(?P<key>[a-z0-9_]+)"[^>]*)>'
)

VALUE = re.compile(r"^      (?P<key>[a-z0-9_]+): '(?P<value>(?:[^'\\]|\\.)*)',?$", re.M)

# What may appear inside a translatable element and still be replaced wholesale.
SIMPLE = re.compile(r"^(?:[^<>]|<strong>|</strong>|<br\s*/?>)*$")


def english() -> dict[str, str]:
    """The English dictionary, as it is written in the script."""
    text = SCRIPT.read_text(encoding="utf-8")
    starts = [m.start() for m in re.finditer(r"^    (?:en|fr|ru): \{", text, re.M)]
    if len(starts) != 3:
        sys.exit(f"expected three language blocks in {SCRIPT}, found {len(starts)}")
    block = text[starts[0] : starts[1]]
    return {
        m.group("key"): m.group("value").replace("\\'", "'").replace("\\\\", "\\")
        for m in VALUE.finditer(block)
    }


def rewrite(page_text: str, words: dict[str, str], report: list[str], name: str) -> str:
    """Replace the body of every translatable element, scanning left to right."""
    out: list[str] = []
    at = 0
    while True:
        opening = OPENING.search(page_text, at)
        if not opening:
            out.append(page_text[at:])
            return "".join(out)

        key = opening.group("key")
        closing_tag = f"</{opening.group('tag')}>"
        close_at = page_text.find(closing_tag, opening.end())
        if close_at == -1:
            report.append(f"{name}: {key} has no closing tag")
            out.append(page_text[at : opening.end()])
            at = opening.end()
            continue

        body = page_text[opening.end() : close_at]
        out.append(page_text[at : opening.end()])

        if key not in words:
            report.append(f"{name}: {key} has no English value in script.js")
            out.append(body)
        elif not SIMPLE.match(body):
            report.append(f"{name}: {key} holds nested markup and was left alone")
            out.append(body)
        else:
            out.append(words[key])

        at = close_at
        out.append("")


# The JSON-LD FAQ block repeats the same questions and answers a third time, for search engines.
# Machine-readable and human-readable copies of one sentence drift the same way the other two did,
# and this one drifts invisibly because nobody reads it on the page.
JSONLD_PAIR = re.compile(
    r'("name": ")(?P<question>(?:[^"\\]|\\.)*)'
    r'(",\s*"acceptedAnswer": \{\s*"@type": "Answer",\s*"text": ")'
    r'(?P<answer>(?:[^"\\]|\\.)*)(")'
)


def as_json(value: str) -> str:
    """The value as it must appear inside a JSON string."""
    return value.replace("\\", "\\\\").replace('"', '\\"')


def sync_jsonld(page_text: str, words: dict[str, str], report: list[str], name: str) -> str:
    """Make each JSON-LD answer the same sentence as the FAQ key it repeats.

    Matched by question rather than by position, so a reordered FAQ is reported instead of being
    silently paired with the wrong answer.
    """
    questions = {
        as_json(words[key]): key.replace("_q", "_a")
        for key in words
        if re.fullmatch(r"faq_q\d+", key)
    }

    def swap(match: re.Match[str]) -> str:
        answer_key = questions.get(match.group("question"))
        if answer_key is None:
            report.append(f"{name}: a JSON-LD question matches no faq_q* value")
            return match.group(0)
        if answer_key not in words:
            report.append(f"{name}: {answer_key} has no English value in script.js")
            return match.group(0)
        return (
            match.group(1)
            + match.group("question")
            + match.group(3)
            + as_json(words[answer_key])
            + match.group(5)
        )

    return JSONLD_PAIR.sub(swap, page_text)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="report drift, write nothing")
    arguments = parser.parse_args()

    words = english()
    problems: list[str] = []
    drifted: list[str] = []

    for page in PAGES:
        original = page.read_text(encoding="utf-8")
        updated = rewrite(original, words, problems, page.name)
        updated = sync_jsonld(updated, words, problems, page.name)
        if updated == original:
            continue
        drifted.append(page.name)
        if not arguments.check:
            page.write_text(updated, encoding="utf-8", newline="\n")

    for problem in problems:
        print(f"error: {problem}")

    if arguments.check:
        for page in drifted:
            print(f"error: {page} differs from the English dictionary")
        total = len(problems) + len(drifted)
        print(f"sync-site-i18n: {len(words)} English value(s), {total} problem(s)")
        return 1 if total else 0

    for page in drifted:
        print(f"updated {page}")
    print(f"sync-site-i18n: {len(words)} English value(s), {len(drifted)} page(s) updated")
    return 1 if problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
