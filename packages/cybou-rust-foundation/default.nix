# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

{
  lib,
  rustPlatform,
}:

rustPlatform.buildRustPackage {
  pname = "cybou-rust-foundation";
  version = "0.1.0";
  src = lib.cleanSourceWith {
    src = ../..;
    filter =
      path: _type:
      !(builtins.elem (baseNameOf path) [
        ".git"
        "dist"
        "node_modules"
        "target"
      ]);
  };

  cargoLock.lockFile = ../../Cargo.lock;
  cargoBuildFlags = [ "--workspace" ];
  cargoTestFlags = [ "--workspace" ];

  # R0 proves that the shared native workspace is reproducible without replacing any installed
  # owner. The browser target is checked separately in CI until the final WASM package is added.
  doCheck = true;

  installPhase = ''
    runHook preInstall
    gateway=$(find target -type f -path '*/release/cybou-web-gateway' -print -quit)
    test -n "$gateway"
    install -Dm755 "$gateway" $out/bin/cybou-web-gateway
    runHook postInstall
  '';

  meta = {
    description = "Cybou Rust protocol, web-contract, and Living Canvas foundation";
    license = lib.licenses.mit;
    platforms = lib.platforms.linux;
  };
}
