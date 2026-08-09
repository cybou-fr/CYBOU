# SPDX-FileCopyrightText: 2026 Stanislav Saveliev
# SPDX-License-Identifier: MIT
#
# M5: eight real process boundaries plus the QML Presence proxy.
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
      test -f "$m/$f" || {
        echo "missing $m/$f"
        exit 1
      }
    done
    grep -q '^module org.cybou.presence$' "$m/qmldir"

    for daemon in \
      cybou-eventd \
      cybou-lifecycled \
      cybou-identityd \
      cybou-intentiond \
      cybou-predictord \
      cybou-selfd \
      cybou-workspaced \
      cybou-presenced; do
      test -x "$out/bin/$daemon" || {
        echo "missing $out/bin/$daemon"
        exit 1
      }
    done

    mkdir -p "$out/share/dbus-1/services"

    install_dbus_service() {
      name="$1"
      binary="$2"
      unit="$3"

      printf '%s\n' \
        '[D-BUS Service]' \
        "Name=$name" \
        "Exec=$out/bin/$binary" \
        "SystemdService=$unit" \
        > "$out/share/dbus-1/services/$name.service"
    }

    install_dbus_service org.cybou.Mind.Event1 cybou-eventd cybou-eventd.service
    install_dbus_service org.cybou.Mind.Lifecycle1 cybou-lifecycled cybou-lifecycled.service
    install_dbus_service org.cybou.Mind.Identity1 cybou-identityd cybou-identityd.service
    install_dbus_service org.cybou.Mind.Intention1 cybou-intentiond cybou-intentiond.service
    install_dbus_service org.cybou.Mind.Predictor1 cybou-predictord cybou-predictord.service
    install_dbus_service org.cybou.Mind.Self1 cybou-selfd cybou-selfd.service
    install_dbus_service org.cybou.Mind.Workspace1 cybou-workspaced cybou-workspaced.service
    install_dbus_service org.cybou.Mind.Presence1 cybou-presenced cybou-presenced.service

    for name in \
      org.cybou.Mind.Event1 \
      org.cybou.Mind.Identity1 \
      org.cybou.Mind.Intention1 \
      org.cybou.Mind.Predictor1 \
      org.cybou.Mind.Self1 \
      org.cybou.Mind.Workspace1 \
      org.cybou.Mind.Presence1; do
      grep -q "^Name=$name$" \
        "$out/share/dbus-1/services/$name.service"
      grep -q '^SystemdService=cybou-.*d.service$' \
        "$out/share/dbus-1/services/$name.service"
    done
  '';

  meta = {
    description = "Cybou process-isolated Mind runtime and Presence QML proxy";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
  };
}
