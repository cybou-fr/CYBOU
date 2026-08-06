// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT

// Cybou Mind Dock - Vertical panel with organ tabs
// Based on Plasma 6.7.3 API

var mindDock = new Panel;
mindDock.location = "right";
mindDock.hiding = "autohide";
mindDock.height = 460;           // Width for vertical panel
mindDock.lengthMode = "fill";    // Stretch along full screen height
mindDock.alignment = "center";
mindDock.floating = true;

// Add the main dock widget that contains all tabs
mindDock.addWidget("org.cybou.presence");
