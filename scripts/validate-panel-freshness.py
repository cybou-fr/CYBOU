#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
"""Refuse a panel that offers to keep itself current and then does not.

`MonitorSignals` carried an `auto_refresh` flag from the day the panel was written. It defaulted to
`true`, nothing in the crate ever read it, and the Monitor fetched its telemetry once when it was
opened and showed that reading until somebody pressed the button. A load average from eleven
minutes ago looks exactly like a load average, which makes this a worse failure than an empty
panel: the desktop was not missing an answer, it was giving an old one confidently.

So the rule is in two halves, and both are needed. A panel whose state carries `auto_refresh` must
install a timer that reads the flag — otherwise the toggle is a lie. And it must render the
freshness controls — otherwise the timer is invisible, and a timer that has silently stopped
(a gateway that went away, a tab hidden for an hour) leaves the panel exactly where it started:
showing something old with nothing on screen admitting it.

The mapping from a card file to its state is by accessor name, taken from `tool_state.rs` rather
than assumed, so renaming a panel's state does not quietly turn this check off.
"""

import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
TOOL_STATE = ROOT / "crates/living-canvas/src/tool_state.rs"
CARDS = ROOT / "crates/living-canvas/src/components/cards"

# `pub struct MonitorSignals {` ... up to the closing brace at column 0.
STRUCT = re.compile(r"pub struct (\w+Signals)\s*\{(.*?)\n\}", re.DOTALL)
# `pub fn monitor(&self, card: CardId) -> MonitorSignals {`
ACCESSOR = re.compile(r"pub fn (\w+)\s*\(\s*&self[^)]*\)\s*->\s*(\w+Signals)")

# What a panel must do with the flag, and what it must show for the flag to mean anything.
INSTALLS_TIMER = re.compile(r"keep_reading\s*\(")
SHOWS_FRESHNESS = re.compile(r"FreshnessControls")


def main() -> int:
    tool_state = TOOL_STATE.read_text(encoding="utf-8")

    offering = {name for name, body in STRUCT.findall(tool_state) if "auto_refresh" in body}
    if not offering:
        # Not a pass. The check exists because this flag existed and did nothing; a build where no
        # panel offers it at all means the rule has stopped watching anything.
        print(
            "error: no panel state carries `auto_refresh`, so this check is watching nothing",
            file=sys.stderr,
        )
        return 1

    accessors = {
        accessor: struct
        for accessor, struct in ACCESSOR.findall(tool_state)
        if struct in offering
    }

    problems = []
    checked = 0
    for path in sorted(CARDS.glob("*.rs")):
        text = path.read_text(encoding="utf-8")
        relative = path.relative_to(ROOT).as_posix()
        for accessor, struct in accessors.items():
            if not re.search(rf"tool_states\s*\.\s*{accessor}\s*\(", text):
                continue
            checked += 1
            if not INSTALLS_TIMER.search(text):
                problems.append(
                    f"error: {relative} uses {struct}, which offers `auto_refresh`, and installs "
                    f"no timer - the toggle promises something nothing does"
                )
            if not SHOWS_FRESHNESS.search(text):
                problems.append(
                    f"error: {relative} refreshes on a timer and shows no age, so a timer that "
                    f"stopped would leave stale readings looking current"
                )

    for problem in problems:
        print(problem, file=sys.stderr)

    if problems:
        print(f"validate-panel-freshness: {checked} panel(s), {len(problems)} unkept promise(s)")
        return 1

    print(
        f"validate-panel-freshness: {checked} panel(s) offer to stay current, "
        f"and every one of them keeps reading and says how old it is"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
