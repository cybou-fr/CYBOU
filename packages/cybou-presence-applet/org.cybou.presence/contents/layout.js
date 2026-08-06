// SPDX-FileCopyrightText: 2026 Cybou contributors
// SPDX-License-Identifier: MIT
//
// Cybou layout configuration for Plasma 6.
//
// This layout uses loadTemplate() for the Mind Dock to provide better isolation
// (as per Panda's analysis in ADR-00XX).

// Load the Cybou Mind Dock template
loadTemplate("org.cybou.plasma.minddock");

// Main top panel
var panel = new Panel;
panel.location = "top";
panel.height = 36;
panel.hiding = "none";
panel.alignment = "center";

// Add widgets to top panel
panel.addWidget("org.kde.plasma.systemtray");
panel.addWidget("org.kde.plasma.digital-clock");
panel.addWidget("org.kde.plasma.pager");
panel.addWidget("org.kde.plasma.taskmanager");
panel.addWidget("org.kde.plasma.systemmonitor");
