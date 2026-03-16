{
  description = "oklch-color-picker — GUI color picker + Lua parser module";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };

        # ---------- runtime libs that eframe/winit/glutin dlopen ----------
        rpathLibs = pkgs.lib.optionals pkgs.stdenv.hostPlatform.isLinux [
          pkgs.libGL # libGL.so, libEGL.so (mesa)
          pkgs.libxkbcommon # libxkbcommon.so
          pkgs.xorg.libX11 # libX11.so
          pkgs.xorg.libXcursor # libXcursor.so
          pkgs.xorg.libXrandr # libXrandr.so
          pkgs.xorg.libXi # libXi.so
          pkgs.xorg.libxcb # libxcb.so
          pkgs.wayland # libwayland-client.so, libwayland-egl.so
        ];

        # ---------- common metadata ----------
        version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
        src = pkgs.lib.cleanSource ./.;

        # ---------- native build inputs (both packages) ----------
        commonNativeBuildInputs = [
          pkgs.pkg-config
        ];

        # ---------- build inputs (both packages) ----------
        commonBuildInputs =
          rpathLibs
          ++ pkgs.lib.optionals pkgs.stdenv.hostPlatform.isDarwin [
            pkgs.darwin.apple_sdk.frameworks.AppKit
            pkgs.darwin.apple_sdk.frameworks.OpenGL
          ];
      in
      {
        packages = {
          # ======================== Binary (GUI app) ========================
          oklch-color-picker = pkgs.rustPlatform.buildRustPackage {
            pname = "oklch-color-picker";
            inherit version src;

            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = commonNativeBuildInputs;
            buildInputs = commonBuildInputs;

            # Only build the binary target, skip the cdylib
            cargoBuildFlags = [
              "--bin"
              "oklch-color-picker"
            ];
            cargoTestFlags = [
              "--bin"
              "oklch-color-picker"
            ];

            # Patch the binary to find dlopen'd libraries at runtime.
            # patchelf --add-rpath bakes library paths into the ELF binary,
            # which is cleaner than makeWrapper + LD_LIBRARY_PATH.
            # dontPatchELF prevents Nix's automatic patchelf from shrinking rpath.
            dontPatchELF = true;
            postInstall = pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
              patchelf --add-rpath "${pkgs.lib.makeLibraryPath rpathLibs}" \
                $out/bin/oklch-color-picker
            '';

            meta = {
              description = "Standalone graphical color picker using the Oklch color space";
              homepage = "https://github.com/eero-lehtinen/oklch-color-picker";
              license = pkgs.lib.licenses.mit;
              mainProgram = "oklch-color-picker";
            };
          };

          # ==================== cdylib (Lua module) ====================
          parser-lua-module = pkgs.rustPlatform.buildRustPackage {
            pname = "parser-lua-module";
            inherit version src;

            cargoLock.lockFile = ./Cargo.lock;

            nativeBuildInputs = commonNativeBuildInputs;
            buildInputs = commonBuildInputs;

            # Only build the library target, skip the binary
            cargoBuildFlags = [ "--lib" ];
            cargoTestFlags = [ "--lib" ];

            # cargoInstallHook copies the cdylib to $out/lib automatically.
            # Neovim's LuaJIT expects `parser_lua_module.so` (no "lib" prefix,
            # always .so even on macOS). Install to lib/lua/5.1/ so it can be
            # added to package.cpath.
            postInstall =
              ''
                mkdir -p $out/lib/lua/5.1
              ''
              + pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
                cp $out/lib/libparser_lua_module.so \
                   $out/lib/lua/5.1/parser_lua_module.so
              ''
              + pkgs.lib.optionalString pkgs.stdenv.hostPlatform.isDarwin ''
                cp $out/lib/libparser_lua_module.dylib \
                   $out/lib/lua/5.1/parser_lua_module.so
              '';

            meta = {
              description = "Oklch color parser as a Lua module (for Neovim)";
              homepage = "https://github.com/eero-lehtinen/oklch-color-picker";
              license = pkgs.lib.licenses.mit;
            };
          };

          default = self.packages.${system}.oklch-color-picker;
        };

        # ======================== Dev shell ========================
        devShells.default = pkgs.mkShell {
          inputsFrom = [
            self.packages.${system}.oklch-color-picker
          ];

          packages = [
            pkgs.rust-analyzer
            pkgs.clippy
            pkgs.rustfmt
          ];

          # In the dev shell, set LD_LIBRARY_PATH so `cargo run` can find
          # the dlopen'd libraries without an rpath patch.
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath rpathLibs;
        };
      }
    )
    // {
      overlays.default = final: _prev: {
        oklch-color-picker = self.packages.${final.system}.oklch-color-picker;
        oklch-color-picker-lua-module = self.packages.${final.system}.parser-lua-module;
      };
    };
}
