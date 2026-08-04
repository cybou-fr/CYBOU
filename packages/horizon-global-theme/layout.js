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
    var d = allDesktops[i];
    d.wallpaperPlugin = "org.kde.image";
    d.currentConfigGroup = ["Wallpaper", "org.kde.image", "General"];
    // A bare package name gives "unknown wallpaper provider type" in the journal and a
    // fallback wallpaper. The provider wants a file URL, so point at the installed SVG.
    d.writeConfig("Image", "file:///run/current-system/sw/share/wallpapers/CybouHorizonDark/contents/images/3840x2160.svg");
    d.writeConfig("FillMode", 2);
}

// No desktop icons (CYB-022, docs/04): the Folder View containment is what puts them there,
// so the desktop stays on the plain Desktop containment. The right-click menu is unaffected.
for (var j = 0; j < allDesktops.length; j++) {
    allDesktops[j].currentConfigGroup = ["General"];
    allDesktops[j].writeConfig("showToolbox", false);
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

// org.kde.plasma.taskmanager, NOT org.kde.plasma.icontasks: in nixpkgs 26.05 the icontasks
// package ships without ui/main.qml, plasmashell fails to build the panel, and the session
// comes up as a black screen. Found in the guest journal:
//   Could not find required file "mainscript" for package ".../org.kde.plasma.icontasks/"
// Icon-only behaviour is a setting here, not a separate applet.
var tasks = panel.addWidget("org.kde.plasma.taskmanager");
tasks.currentConfigGroup = ["General"];
tasks.writeConfig("onlyGroupWhenFull", false);
tasks.writeConfig("iconOnly", true);
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
