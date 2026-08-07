// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

var mindDock = new Panel;
mindDock.location = "right";
// Plasma calls this `height`; on a vertical panel it is the width.
mindDock.height = 420;
mindDock.lengthMode = "fill";
mindDock.alignment = "center";
// Keep it visible while the Mind UI is under active development.
mindDock.hiding = "none";
mindDock.floating = true;
mindDock.addWidget("org.cybou.presence");
