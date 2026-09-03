#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
"""One WebSocket client, by hand, so the gate needs nothing installed.

It opens `/api/v1/terminal` with the session cookie, asks for a terminal, types one command and
reads what comes back — the same exchange the owner was driven through directly, this time through
the gateway that a browser talks to. CBOR is written and read by hand too: the frames here are three
shapes and a length, and a dependency for that would be a dependency the gate has to install on
every host it runs on.
"""

import base64
import http.client
import os
import socket
import struct
import sys

BASE_HOST = "127.0.0.1"


def cbor_text(value: bytes) -> bytes:
    return bytes([0x60 + len(value)]) + value if len(value) < 24 else bytes([0x78, len(value)]) + value


def cbor_bytes(value: bytes) -> bytes:
    if len(value) < 24:
        return bytes([0x40 + len(value)]) + value
    if len(value) < 256:
        return bytes([0x58, len(value)]) + value
    return bytes([0x59]) + struct.pack(">H", len(value)) + value


def cbor_uint(value: int) -> bytes:
    if value < 24:
        return bytes([value])
    if value < 256:
        return bytes([0x18, value])
    return bytes([0x19]) + struct.pack(">H", value)


def open_frame(columns: int, rows: int) -> bytes:
    """`FromGateway::Open`: externally tagged, so a map of one entry holding a map of two."""
    return (
        b"\xa1"
        + cbor_text(b"open")
        + b"\xa2"
        + cbor_text(b"columns")
        + cbor_uint(columns)
        + cbor_text(b"rows")
        + cbor_uint(rows)
    )


def input_frame(keys: bytes) -> bytes:
    """`FromGateway::Input`: a newtype variant wrapping the bytes."""
    return b"\xa1" + cbor_text(b"input") + cbor_bytes(keys)


def mask(payload: bytes) -> bytes:
    """One client frame: binary, final, masked as every client frame must be."""
    key = os.urandom(4)
    masked = bytes(byte ^ key[index % 4] for index, byte in enumerate(payload))
    header = b"\x82"
    if len(payload) < 126:
        header += bytes([0x80 | len(payload)])
    else:
        header += bytes([0x80 | 126]) + struct.pack(">H", len(payload))
    return header + key + masked


def read_exact(sock: socket.socket, count: int) -> bytes:
    buffer = b""
    while len(buffer) < count:
        chunk = sock.recv(count - len(buffer))
        if not chunk:
            raise EOFError("the gateway closed the socket")
        buffer += chunk
    return buffer


def read_frame(sock: socket.socket) -> tuple[int, bytes]:
    first, second = read_exact(sock, 2)
    opcode = first & 0x0F
    length = second & 0x7F
    if length == 126:
        length = struct.unpack(">H", read_exact(sock, 2))[0]
    elif length == 127:
        length = struct.unpack(">Q", read_exact(sock, 8))[0]
    return opcode, read_exact(sock, length)


def decode(data: bytes, at: int = 0):
    """Enough CBOR to read what the owner sends: maps, text, arrays, integers and null.

    The owner's `Output` carries a `Vec<u8>`, which ciborium writes as an array of integers rather
    than as a byte string, so a probe looking for a byte string would find nothing and report a
    terminal that had in fact answered.
    """
    initial = data[at]
    major, extra = initial >> 5, initial & 0x1F
    at += 1
    if extra < 24:
        value = extra
    elif extra == 24:
        value, at = data[at], at + 1
    elif extra == 25:
        value, at = struct.unpack_from(">H", data, at)[0], at + 2
    elif extra == 26:
        value, at = struct.unpack_from(">I", data, at)[0], at + 4
    elif extra == 27:
        value, at = struct.unpack_from(">Q", data, at)[0], at + 8
    else:
        value = None

    if major == 0:
        return value, at
    if major == 1:
        return -1 - value, at
    if major in (2, 3):
        raw = data[at : at + value]
        return (raw if major == 2 else raw.decode()), at + value
    if major == 4:
        items = []
        for _ in range(value):
            item, at = decode(data, at)
            items.append(item)
        return items, at
    if major == 5:
        entries = {}
        for _ in range(value):
            key, at = decode(data, at)
            entry, at = decode(data, at)
            entries[key] = entry
        return entries, at
    if major == 7:
        return {20: False, 21: True, 22: None, 23: None}.get(extra), at
    raise ValueError(f"unhandled CBOR major type {major}")


def owner_output(frame: bytes) -> bytes:
    """The bytes in one `FromOwner::Output`, or nothing for any other frame."""
    value, _ = decode(frame)
    if isinstance(value, str):
        return b""
    payload = value.get("output") if isinstance(value, dict) else None
    if isinstance(payload, bytes):
        return payload
    if isinstance(payload, list):
        return bytes(payload)
    return b""


def main() -> int:
    port, cookie, expected_uid = sys.argv[1], sys.argv[2], sys.argv[3]
    connection = http.client.HTTPConnection(BASE_HOST, int(port), timeout=15)
    connection.request(
        "GET",
        "/api/v1/terminal",
        headers={
            "Host": f"{BASE_HOST}:{port}",
            "Connection": "Upgrade",
            "Upgrade": "websocket",
            "Sec-WebSocket-Version": "13",
            "Sec-WebSocket-Key": base64.b64encode(os.urandom(16)).decode(),
            "Cookie": cookie,
        },
    )
    response = connection.getresponse()
    if response.status != 101:
        print(f"the gateway answered {response.status} rather than upgrading", file=sys.stderr)
        return 1
    sock = connection.sock
    sock.settimeout(15)

    sock.sendall(mask(open_frame(80, 24)))
    sock.sendall(mask(input_frame(b"id -u; exit\n")))

    seen = b""
    while True:
        try:
            opcode, payload = read_frame(sock)
        except (EOFError, TimeoutError, socket.timeout):
            break
        if opcode == 0x8:
            break
        if opcode != 0x2:
            continue
        seen += owner_output(payload)
        if expected_uid.encode() in seen:
            print("the gateway carried a terminal that ran as uid " + expected_uid)
            return 0
    print(f"the uid never came back through the gateway; saw {seen!r}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
