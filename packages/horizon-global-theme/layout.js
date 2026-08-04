// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: CC-BY-SA-4.0
//
// Cybou Horizon default layout.
//
// The file name is fixed by Plasma and is NOT derived from the package id: it must be
// org.kde.plasma.desktop-layout.js inside contents/layouts/. Any other name is ignored
// silently - no panel, no error. Verified against plasma-workspace/lookandfeel/org.kde.breezedark.
//
// Geometry comes from spec/design-tokens.json: top edge, 44 px, 12 px margin, floating.
// Applet order comes from docs/04-desktop-layout.md.

// NEVER name this variable `desktops`. `var` hoists, so the name shadows the desktops()
// function for the whole scope and the call throws before anything is created - the layout
// dies silently and the session comes up as a black screen with no containment. That is
// exactly what shipped on 2026-08-04 and it cost a full image rebuild to find.
var allDesktops = desktops();
for (var i = 0; i < allDesktops.length; i++) {
    allDesktops[i].wallpaperPlugin = "org.kde.image";
    allDesktops[i].currentConfigGroup = ["Wallpaper", "org.kde.image", "General"];
    allDesktops[i].writeConfig("Image", "CybouHorizonDark");
}

var panel = new Panel;
panel.location = "top";
panel.height = 44;
panel.floating = true;

// Launcher first, carrying the Cybou Aperture mark. The icon lives in the applet's own
// configuration, not in the theme, which is why it can only be set here.
var launcher = panel.addWidget("org.kde.plasma.kickoff");
launcher.currentConfigGroup = ["General"];
launcher.writeConfig("icon", "cybou");

// Pager: four desktops, compact, active one carries the accent (docs/04).
panel.addWidget("org.kde.plasma.pager");

// Icons only, no window titles - the panel is an anchor, not a task list.
var tasks = panel.addWidget("org.kde.plasma.icontasks");
tasks.currentConfigGroup = ["General"];
tasks.writeConfig("launchers", [
    "applications:systemsettings.desktop",
    "applications:org.kde.dolphin.desktop",
    "applications:firefox.desktop",
    "applications:org.kde.konsole.desktop"
]);

panel.addWidget("org.kde.plasma.marginsseparator");
panel.addWidget("org.kde.plasma.systemtray");

var clock = panel.addWidget("org.kde.plasma.digitalclock");
clock.currentConfigGroup = ["Appearance"];
// HH:mm, but the calendar popup keeps locale behaviour - docs/04 forbids forcing a
// 12- or 24-hour clock globally.
clock.writeConfig("showDate", false);
