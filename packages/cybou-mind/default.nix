# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# Mind runtime package. M3 installs both the Presence QML module and the D-Bus-activated
# cybou-eventd executable. eventd is the normal production owner of journal.db.
{
  lib,
  stdenv,
  cmake,
  ninja,
  dbus,
  qt6,
}:
stdenv.mkDerivation {
  pname = "cybou-mind";
  version = "0.1.0";

  src = ../../mind;

  nativeBuildInputs = [
    cmake
    ninja
    dbus
    qt6.wrapQtAppsHook
  ];

  buildInputs = [
    qt6.qtbase
    qt6.qtdeclarative
  ];

  cmakeFlags = [
    (lib.cmakeFeature "CMAKE_BUILD_TYPE" "Release")
    (lib.cmakeFeature "CYBOU_QML_INSTALL_DIR" "${placeholder "out"}/${qt6.qtbase.qtQmlPrefix}")
  ];

  doCheck = true;
  checkPhase = ''
    runHook preCheck
    QT_QPA_PLATFORM=offscreen ctest --output-on-failure
    runHook postCheck
  '';

  postInstall = ''
    m=$out/${qt6.qtbase.qtQmlPrefix}/org/cybou/presence
    for f in qmldir libcybou-presence-qml.so; do
      test -f "$m/$f" || { echo "missing $m/$f"; exit 1; }
    done
    grep -q '^module org.cybou.presence$' "$m/qmldir"

    test -x "$out/bin/cybou-eventd" || {
      echo "missing $out/bin/cybou-eventd"
      exit 1
    }

    mkdir -p "$out/share/dbus-1/services"
    printf '%s\n' \
      '[D-BUS Service]' \
      'Name=org.cybou.Mind.Event1' \
      "Exec=$out/bin/cybou-eventd" \
      > "$out/share/dbus-1/services/org.cybou.Mind.Event1.service"

    grep -q '^Name=org.cybou.Mind.Event1$' \
      "$out/share/dbus-1/services/org.cybou.Mind.Event1.service"
  '';

  meta = {
    description = "Cybou Mind runtime, Presence QML module, and single-writer eventd";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
  };
}
