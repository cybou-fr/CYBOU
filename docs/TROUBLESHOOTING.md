<!--
SPDX-FileCopyrightText: 2026 Cybou contributors
SPDX-License-Identifier: MIT
-->

# Troubleshooting

## QML configure errors

Use relative QML resource paths or explicit resource aliases. Avoid duplicate visual QML trees.

## `org.cybou.presence` not installed

Verify the QML plugin, exact URI, runtime import path, and Plasma applet package.

## Presence blank

Check Journal path and permissions, QML loading, `plasmashell` logs, awake state, and schema compatibility.

## Database locked

Avoid multiple Presence instances. The target fix is a single-writer eventd.

## VM black screen

Use safe graphics only when required. Do not force software rendering on normal hardware.

## Repairing Plasma

Back up Plasma configuration and do not delete cognitive state while resetting the desktop.
