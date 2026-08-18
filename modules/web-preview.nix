# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT
#
# Static preview surface for the remote evaluation host (docs/DEPLOYMENT.md).
#
# What this is: the committed `www/` site served over plain HTTP so a browser on another machine
# can look at it. What this is not: the web-first Presence surface. `cybou-web-gateway` and the
# Rust/WASM Living Canvas described in docs/WEB_UI_ARCHITECTURE.md do not exist yet, so nothing
# here talks to Presence1, holds a session, or accepts a mutation. When the W1 gateway arrives it
# gets its own module and its own security review; it must not be smuggled in as "the preview
# host already serves HTTP".
{ pkgs, ... }:
let
  # cleanSource keeps editor droppings and VCS metadata out of the served root.
  site = pkgs.lib.cleanSource ../www;
in
{
  services.nginx = {
    enable = true;
    recommendedGzipSettings = true;
    recommendedOptimisation = true;

    # No TLS, and therefore no session, cookie, or credential may ever be introduced on this
    # listener. Remote access to real projections waits for phase W4, which requires TLS at the
    # external boundary (docs/WEB_UI_ARCHITECTURE.md).
    virtualHosts."cybou-preview" = {
      default = true;
      root = "${site}";

      extraConfig = ''
        add_header X-Content-Type-Options nosniff always;
        add_header X-Frame-Options DENY always;
        add_header Referrer-Policy no-referrer always;
      '';
    };
  };
}
