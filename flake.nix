{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nci.url = "github:yusdacra/nix-cargo-integration";
    nci.inputs.nixpkgs.follows = "nixpkgs";
    parts.url = "github:hercules-ci/flake-parts";
    parts.inputs.nixpkgs-lib.follows = "nixpkgs";
    devshell.url = "github:numtide/devshell";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = inputs @ {
    parts,
    nci,
    devshell,
    rust-overlay,
    nixpkgs,
    ...
  }:
    parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux"];
      imports = [nci.flakeModule parts.flakeModules.easyOverlay devshell.flakeModule];
      perSystem = {
        config,
        pkgs,
        system,
        lib,
        ...
      }: let
        crateName = "ayaya-discord-bot";
        # shorthand for accessing this crate's outputs
        # you can access crate outputs under `config.nci.outputs.<crate name>` (see documentation)
        crateOutputs = config.nci.outputs.${crateName};
        libPath = with pkgs;
          lib.makeLibraryPath
          [
            libGL
            libxkbcommon
            openssl.dev
            wayland
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
            libopus
            openssl
          ];
      in {
        # use oxalica/rust-overlay
        _module.args.pkgs = import nixpkgs {
          inherit system;
          overlays = [rust-overlay.overlays.default];
          config.allowUnfree = true;
        };
        nci = {
          # relPath is empty to denote current dir
          projects.${crateName}.path = ./.;

          crates.${crateName} = {
            # export crate (packages and devshell) in flake outputs
            export = true;

            # overrides
            drvConfig = {
              mkDerivation = {
                nativeBuildInputs = [pkgs.wayland-protocols pkgs.makeWrapper pkgs.libxkbcommon];
                buildInputs = [pkgs.pkg-config pkgs.openssl.dev pkgs.openssl pkgs.perl pkgs.libopus pkgs.openssl pkgs.clang pkgs.mold];
              };
            };

            # dependency overrides
            depsDrvConfig = {
              mkDerivation = {
                nativeBuildInputs = [pkgs.wayland-protocols pkgs.libxkbcommon];
                buildInputs = [pkgs.pkg-config pkgs.openssl.dev pkgs.openssl pkgs.perl pkgs.cmake pkgs.libopus pkgs.openssl pkgs.clang pkgs.mold];
              };
            };
            runtimeLibs = with pkgs; [
              libGL
              libxkbcommon
              openssl.dev
              wayland
              xorg.libX11
              xorg.libXcursor
              xorg.libXi
              xorg.libXrandr
              libopus
            ];
          };

          toolchains = {
            build =
              pkgs.rust-bin.stable.latest.minimal;
          };
        };

        # use numtide/devshell
        devshells.default = with pkgs; {
          motd = ''
            -----------------
            -ayaya-discord-bot devshell-
            -----------------
            $(type -p menu &>/dev/null && menu)
          '';
          env = [
            {
              name = "LD_LIBRARY_PATH";
              value = libPath;
            }
            {
              name = "PKG_CONFIG_PATH";
              value = "${pkgs.libxkbcommon.dev}/lib/pkgconfig:${pkgs.openssl.dev}/lib/pkgconfig:${pkgs.libopus.dev}/lib/pkgconfig";
            }
          ];

          packages = [
            (rust-bin.stable.latest.default.override {
              extensions = ["rust-src" "rust-analyzer"];
            })
            just
            pkg-config
            cmake
            ripgrep
            mold
            libopus
            yt-dlp
          ];

          packagesFrom = [crateOutputs.packages.release];

          commands = [
            {
              name = "nix-run-${crateName}";
              command = "RUST_LOG=debug nix run .#${crateName}-dev";
              help = "Run ${crateName} (debug build)";
              category = "Run";
            }
            {
              name = "nix-run-${crateName}-rel";
              command = "RUST_LOG=debug nix run .#${crateName}-rel";
              help = "Run ${crateName} (release build)";
              category = "Run";
            }
            {
              name = "nix-build-${crateName}";
              command = "RUST_LOG=debug nix build .#${crateName}-dev";
              help = "Build ${crateName} (debug build)";
              category = "Build";
            }
            {
              name = "nix-build-${crateName}-rel";
              command = "RUST_LOG=debug nix build .#${crateName}-rel";
              help = "Build ${crateName} (release build)";
              category = "Build";
            }
          ];
        };

        # export the release package of the crate as default package
        packages.default = crateOutputs.packages.release;

        # export overlay using easyOverlays
        overlayAttrs = {
          inherit (config.packages) ayaya-discord-bot;
          /*
          inherit (inputs.rust-overlay.overlays) default;
          */
        };
        packages.ayaya-discord-bot = crateOutputs.packages.release;
        packages.ayaya-discord-bot-release = crateOutputs.packages.release;
        packages.ayaya-discord-bot-debug = crateOutputs.packages.release;
      };
      flake = {
        homeManagerModules = {
          ayaya-discord-bot = import ./nix/hm-module.nix inputs.self;
          default = inputs.self.homeManagerModules.ayaya-discord-bot;
        };
      };
    };
}
