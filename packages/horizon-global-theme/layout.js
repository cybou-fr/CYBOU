// SPDX-FileCopyrightText: 2026 Stanislav Saveliev
// SPDX-License-Identifier: CC-BY-SA-4.0

var allDesktops = desktops();
for (var i = 0; i < allDesktops.length; i++) {
    var d = allDesktops[i];
    d.wallpaperPlugin = "org.kde.image";
    d.currentConfigGroup = ["Wallpaper", "org.kde.image", "General"];
    d.writeConfig("Image", "file:///run/current-system/sw/share/wallpapers/CybouHorizonDark/contents/images/3840x2160.svg");
    d.writeConfig("FillMode", 2);
}
for (var j = 0; j < allDesktops.length; j++) {
    allDesktops[j].currentConfigGroup = ["General"];
    allDesktops[j].writeConfig("showToolbox", false);
}

var panel = new Panel;
panel.location = "top";
panel.height = 44;
panel.floating = true;

var launcher = panel.addWidget("org.kde.plasma.kickoff");
launcher.currentConfigGroup = ["General"];
launcher.writeConfig("icon", "cybou");

panel.addWidget("org.kde.plasma.pager");

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
clock.writeConfig("showDate", false);

// Presence is not a compact top-panel popup. Load its dedicated side panel.
loadTemplate("org.cybou.plasma.minddock");
