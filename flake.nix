{
  description = "Dagger Explorer — egui file explorer for NixOS";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      crane,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        inherit (pkgs) lib;

        # winit dlopens Wayland/X11/Vulkan at runtime on Linux; these must be on LD_LIBRARY_PATH.
        guiRuntimeLibs = with pkgs; [
          libGL
          libxkbcommon
          vulkan-loader
          fontconfig
          freetype
          wayland
          at-spi2-core
          libx11
          libxcursor
          libxi
          libxrandr
          libxcb
        ];

        rustToolchain = pkgs.rust-bin.stable.latest.default;

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        commonArgs = {
          pname = "dagger-explorer";
          version = "0.1.0";
          src = ./.;
          nativeBuildInputs = with pkgs; [
            pkg-config
            rustToolchain
            makeWrapper
          ];
          buildInputs = with pkgs; [
            openssl
          ];
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        dagger-explorer = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
            doCheck = false;
            postInstall = ''
              wrapProgram "$out/bin/dagger-explorer" \
                --prefix LD_LIBRARY_PATH : "${lib.makeLibraryPath guiRuntimeLibs}"
            '';
          }
        );
      in
      {
        packages.default = dagger-explorer;
        packages.dagger-explorer = dagger-explorer;

        apps.default = flake-utils.lib.mkApp {
          drv = dagger-explorer;
        };

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rustToolchain
            rust-analyzer
            cargo-watch
            pkg-config
            openssl
            curl
            unzip
            ffmpeg
          ];

          buildInputs = guiRuntimeLibs;

          shellHook = ''
            export LD_LIBRARY_PATH="${
              lib.makeLibraryPath guiRuntimeLibs
            }''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
            export RUST_SRC_PATH="${pkgs.rustPlatform.rustLibSrc}"
          '';
        };

        formatter = pkgs.nixfmt-rfc-style;
      }
    );
}
