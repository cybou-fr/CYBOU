# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# The Mind, built as its own derivation.
#
# ADR-0008 keeps `mind/` isolated so it can be developed without disturbing the desktop. What
# it produces here is a single QML module: the applet instantiates Presence in-process inside
# plasmashell, so there is no daemon, no D-Bus name and nothing listening. The journal lives in
# the user's data directory and never leaves it.
#
# The tests run at build time. They are the only gate that covers C++ in this project, so a
# broken organ must fail the build rather than reach a desktop - `nix flake check` inherits
# this by depending on the package.
{
  lib,
  stdenv,
  cmake,
  ninja,
  qt6,
}:
stdenv.mkDerivation {
  pname = "cybou-mind";
  version = "0.1.0";

  src = ../../mind;

  nativeBuildInputs = [
    cmake
    ninja
    qt6.wrapQtAppsHook
  ];

  buildInputs = [
    qt6.qtbase
    qt6.qtdeclarative
  ];

  cmakeFlags = [
    (lib.cmakeFeature "CMAKE_BUILD_TYPE" "Release")
    # Where plasmashell will look. Passed explicitly rather than inferred, so the path that
    # ends up in the store is the same one the check below asserts on.
    (lib.cmakeFeature "CYBOU_QML_INSTALL_DIR" "${placeholder "out"}/${qt6.qtbase.qtQmlPrefix}")
  ];

  doCheck = true;
  # QtTest needs a platform plugin even for headless organs; offscreen is the one that exists
  # in a sandbox with no display.
  checkPhase = ''
    runHook preCheck
    QT_QPA_PLATFORM=offscreen ctest --output-on-failure
    runHook postCheck
  '';

  # A QML module that installed to the wrong place fails silently at runtime - the panel just
  # shows nothing. Assert the layout here, where it is still a build failure.
  postInstall = ''
    m=$out/${qt6.qtbase.qtQmlPrefix}/org/cybou/presence
    for f in qmldir libcybou-presence-qml.so; do
      test -f "$m/$f" || { echo "missing $m/$f"; exit 1; }
    done
    grep -q '^module org.cybou.presence$' "$m/qmldir"
  '';

  meta = {
    description = "Cybou Mind: the Presence organs and their QML module";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
  };
}
