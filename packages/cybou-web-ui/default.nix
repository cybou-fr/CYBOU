# SPDX-FileCopyrightText: 2026 Cybou contributors
# SPDX-License-Identifier: MIT

{
  lib,
  rustPlatform,
  trunk,
  wasm-bindgen-cli_0_2_126,
  lld,
}:

rustPlatform.buildRustPackage {
  pname = "cybou-web-ui";
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
  nativeBuildInputs = [
    trunk
    wasm-bindgen-cli_0_2_126
    lld
  ];

  buildPhase = ''
    runHook preBuild
    export HOME="$TMPDIR/cybou-web-ui-home"
    # The cross stdenv defaults to its C compiler driver. Browser WASM must instead be linked
    # directly by wasm-ld; otherwise wasm-bindgen export names are parsed as input filenames.
    export CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER="${lld}/bin/wasm-ld"
    mkdir -p "$HOME"
    cd crates/living-canvas
    trunk build --release
    cd ../..
    runHook postBuild
  '';

  doCheck = false;

  installPhase = ''
    runHook preInstall
    mkdir -p "$out/share/cybou/web"
    cp -r target/living-canvas/. "$out/share/cybou/web/"
    runHook postInstall
  '';

  meta = {
    description = "Immutable Rust/WASM Living Canvas frontend";
    license = lib.licenses.mit;
  };
}
