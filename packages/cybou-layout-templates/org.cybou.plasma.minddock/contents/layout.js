// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

// Main inspection surface: native Plasma auto-hide remains the default.
var mindDock = new Panel;
mindDock.location = "right";
// Plasma calls this `height`; on a vertical panel it is the width.
mindDock.height = 420;
mindDock.lengthMode = "fill";
mindDock.alignment = "center";
mindDock.hiding = "autohide";
mindDock.floating = true;
mindDock.addWidget("org.cybou.presence");

// Discoverability surface: a tiny always-visible right-edge handle.
// It is deliberately a separate panel so it remains visible while the main dock is auto-hidden.
var mindHandle = new Panel;
mindHandle.location = "right";
mindHandle.height = 18;
mindHandle.lengthMode = "custom";
mindHandle.length = 82;
mindHandle.alignment = "center";
mindHandle.hiding = "none";
mindHandle.floating = true;

var handleWidget = mindHandle.addWidget("org.cybou.mindhandle");
handleWidget.globalShortcut = "Meta+M";
