# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

{
  cage,
  chromium,
  coreutils,
  curl,
  lib,
  systemd,
  writeShellApplication,
}:

let
  launcher = writeShellApplication {
    name = "cybou-desktop-shell";
    runtimeInputs = [
      chromium
      coreutils
    ];
    text = ''
      : "''${XDG_RUNTIME_DIR:?Cybou desktop requires a user runtime directory}"
      profile="$XDG_RUNTIME_DIR/cybou/chromium-profile"
      mkdir -p "$profile"
      chmod 0700 "$XDG_RUNTIME_DIR/cybou" "$profile"

      exec chromium \
        --ozone-platform=wayland \
        --app=http://127.0.0.1:8787/ \
        --kiosk \
        --user-data-dir="$profile" \
        --no-first-run \
        --no-default-browser-check \
        --disable-background-networking \
        --disable-component-update \
        --disable-default-apps \
        --disable-dev-tools \
        --disable-extensions \
        --disable-features=AutofillServerCommunication,OptimizationHints,Translate \
        --disable-sync \
        --metrics-recording-only \
        --password-store=basic
    '';
  };

  session = writeShellApplication {
    name = "cybou-desktop-session";
    runtimeInputs = [
      cage
      coreutils
      curl
      systemd
    ];
    text = ''
      systemctl --user start cybou-web-gateway.service

      ready=0
      for _attempt in $(seq 1 50); do
        if curl --fail --silent --show-error --max-time 1 \
          http://127.0.0.1:8787/api/v1/session >/dev/null; then
          ready=1
          break
        fi
        sleep 0.1
      done

      if [ "$ready" -ne 1 ]; then
        echo "Cybou gateway did not become ready" >&2
        systemctl --user stop cybou-web-gateway.service
        exit 1
      fi

      cleanup() {
        systemctl --user stop cybou-web-gateway.service || true
      }
      trap cleanup EXIT INT TERM

      cage -- ${launcher}/bin/cybou-desktop-shell
    '';
  };
in
session.overrideAttrs (old: {
  pname = "cybou-desktop-shell";
  passthru = (old.passthru or { }) // {
    providedSessions = [ "cybou" ];
  };
  postInstall = (old.postInstall or "") + ''
    mkdir -p "$out/share/wayland-sessions"
    substitute ${./cybou.desktop} "$out/share/wayland-sessions/cybou.desktop" \
      --replace-fail "@SESSION@" "$out/bin/cybou-desktop-session"
  '';
  meta = {
    description = "Single-surface Chromium/Ozone Cybou desktop preview";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
  };
})
