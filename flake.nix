{
  description = "EGui Flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    esp-dev.url = "github:mirrexagon/nixpkgs-esp-dev";
  };

  outputs = { self, nixpkgs, flake-utils, esp-dev }:
    flake-utils.lib.eachDefaultSystem (system:
      let pkgs = nixpkgs.legacyPackages.${system};
      in {
        devShells.default = pkgs.mkShell rec {
          name = "egui";
          nativeBuildInputs = with pkgs; [ pkg-config libxkbcommon wayland libGL ];

          inputsFrom = [
              esp-dev.devShells.${system}.esp32-idf
          ];

          buildInputs = with pkgs; [
            wayland
            pkg-config
            openssl

            cmake
            ninja
            ccache
            udev

            libclang
            linuxHeaders

            libGL
            libxkbcommon
            vulkan-loader
            glibc.dev
          ];

          shellHook = ''
            export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${
              builtins.toString (pkgs.lib.makeLibraryPath buildInputs)
            }";

            export BINDGEN_EXTRA_CLANG_ARGS="\
              -isystem ${pkgs.linuxHeaders}/include \
              -isystem ${pkgs.glibc.dev}/include"
          '';
        };
      });
}

