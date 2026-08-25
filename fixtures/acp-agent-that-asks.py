#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
"""An ACP agent that streams a reply and asks for permission it will not be given.

A stand-in for a real agent, written to exercise the two things about a turn that are Cybou's rather
than the protocol's: that the agent's words are separated from its thoughts, and that
`session/request_permission` is refused rather than approved by a default.

It answers the protocol faithfully and behaves like an agent that believes asking is how it gets
things done. It records what it was told in the file named by `CYBOU_FAKE_AGENT_LOG`, so the gate can check the
refusal reached the agent rather than only that the client wrote one down.
"""

from __future__ import annotations

import json
import os
import sys

SESSION = "session-from-the-fake-agent"


def send(message: dict) -> None:
    sys.stdout.write(json.dumps(message) + "\n")
    sys.stdout.flush()


def notify(update: dict) -> None:
    send(
        {
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": {"sessionId": SESSION, "update": update},
        }
    )


def note(text: str) -> None:
    """Record what the client answered, where a gate can read it.

    To a file rather than to stderr: a client is entitled to capture an agent's stderr, and
    one that does would leave this fixture unable to report the one thing it exists for.
    """
    path = os.environ.get("CYBOU_FAKE_AGENT_LOG")
    if path is None:
        sys.stderr.write(text + "\n")
        sys.stderr.flush()
        return
    with open(path, "a", encoding="utf-8") as log:
        log.write(text + "\n")


def read() -> dict | None:
    line = sys.stdin.readline()
    if not line:
        return None
    return json.loads(line)


def ask_permission() -> dict:
    """Ask the client for something, and wait for its answer before finishing the turn."""
    send(
        {
            "jsonrpc": "2.0",
            "id": 9001,
            "method": "session/request_permission",
            "params": {
                "sessionId": SESSION,
                "toolCall": {
                    "toolCallId": "call-1",
                    "title": "restart nginx.service",
                },
                "options": [
                    {"optionId": "allow", "name": "Allow once", "kind": "allow_once"},
                    {"optionId": "reject", "name": "Refuse", "kind": "reject_once"},
                ],
            },
        }
    )
    while True:
        message = read()
        if message is None:
            note("permission answer never arrived")
            return {}
        if message.get("id") == 9001:
            return message.get("result") or {}


def main() -> int:
    while True:
        message = read()
        if message is None:
            return 0
        method = message.get("method")
        request_id = message.get("id")

        if method == "initialize":
            send(
                {
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "result": {
                        "protocolVersion": 1,
                        "agentInfo": {"name": "fake-agent", "version": "0.0.1"},
                        "agentCapabilities": {},
                        "authMethods": [],
                    },
                }
            )
        elif method == "session/new":
            send({"jsonrpc": "2.0", "id": request_id, "result": {"sessionId": SESSION}})
        elif method == "session/prompt":
            # A thought and a message, in that order. A client that concatenated both would present
            # the draft as the answer.
            notify(
                {
                    "sessionUpdate": "agent_thought_chunk",
                    "content": {"type": "text", "text": "deciding what to say"},
                }
            )
            notify(
                {
                    "sessionUpdate": "agent_message_chunk",
                    "content": {"type": "text", "text": "the fake agent "},
                }
            )
            notify(
                {
                    "sessionUpdate": "agent_message_chunk",
                    "content": {"type": "text", "text": "answered"},
                }
            )
            outcome = ask_permission()
            note("permission outcome: " + json.dumps(outcome.get("outcome")))
            send(
                {
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "result": {"stopReason": "end_turn"},
                }
            )
        elif request_id is not None:
            send(
                {
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "error": {"code": -32601, "message": "method not found: " + str(method)},
                }
            )


if __name__ == "__main__":
    sys.exit(main())
