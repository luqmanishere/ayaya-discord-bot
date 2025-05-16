{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    fenix.url = "github:nix-community/fenix";
  };
  outputs = {
    self,
    flake-utils,
    naersk,
    nixpkgs,
    fenix,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = (import nixpkgs) {
          inherit system;
        };

        toolchain = with fenix.packages.${system};
          combine [
            stable.toolchain
            targets.aarch64-apple-darwin.stable.rust-std
          ];

        naersk' = pkgs.callPackage naersk {};
      in rec {
        # For `nix build` & `nix run`:
        # defaultPackage = naersk'.buildPackage {
        #   src = ./.;
        # };

        # For `nix develop` (optional, can be skipped):
        # devShells.default = pkgs.mkShell {
        #   nativeBuildInputs = with pkgs; [
        #     toolchain
        #     rust-analyzer
        #     yt-dlp
        #     ffmpeg
        #     cmake
        #     mold
        #   ];
        #   buildInputs = with pkgs; [darwin.apple_sdk.frameworks.SystemConfiguration iconv.dev libopus.dev pkg-config];
        # };
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            toolchain
            yt-dlp
            ffmpeg
            cmake
            mold
            pkg-config
            sea-orm-cli
            just
            git-cliff
          ];
          buildInputs = with pkgs; [iconv.dev libopus.dev cargo-shuttle openssl];
        };
      }
    );
}
