// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

var mindDock = new Panel;
mindDock.location = "right";
// Plasma calls this `height`; on a vertical panel it is the width.
mindDock.height = 420;
mindDock.lengthMode = "fill";
mindDock.alignment = "center";
// Production default: reveal from the right screen edge and hide when the pointer leaves.
mindDock.hiding = "autohide";
mindDock.floating = true;
mindDock.addWidget("org.cybou.presence");
