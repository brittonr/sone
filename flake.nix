{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, rust-overlay, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };

      version = "0.9.0";

      gstDeps = with pkgs.gst_all_1; [
        gstreamer
        gst-plugins-base
        gst-plugins-good
        gst-plugins-bad
        gst-libav
      ];

      libraries = with pkgs; [
        webkitgtk_4_1
        gtk3
        glib
        cairo
        pango
        gdk-pixbuf
        librsvg
        openssl
        alsa-lib
        libsecret
        libayatana-appindicator
      ] ++ gstDeps;

      frontend = pkgs.buildNpmPackage {
        pname = "sone-frontend";
        inherit version;
        src = ./.;
        npmDepsHash = "sha256-pGAEr6KRLuKUE4vJDyK7hI6MRNAafZFNv/i+zUvnng4=";
        buildPhase = ''
          runHook preBuild
          npx tsc && npx vite build
          runHook postBuild
        '';
        installPhase = ''
          runHook preInstall
          cp -r dist $out
          runHook postInstall
        '';
      };
    in
    {
      packages.${system}.default = pkgs.rustPlatform.buildRustPackage {
        pname = "sone";
        inherit version;
        src = ./.;

        cargoRoot = "src-tauri";
        buildAndTestSubdir = "src-tauri";
        cargoLock.lockFile = ./src-tauri/Cargo.lock;
        cargoBuildFlags = [ "--features" "tauri/custom-protocol" ];

        nativeBuildInputs = with pkgs; [
          pkg-config
          gobject-introspection
          wrapGAppsHook3
        ];

        buildInputs = libraries;

        postPatch = ''
          cp -r ${frontend} dist
        '';

        preFixup = ''
          gappsWrapperArgs+=(
            --set GST_PLUGIN_PATH "${pkgs.lib.makeSearchPath "lib/gstreamer-1.0" gstDeps}"
            --prefix GIO_EXTRA_MODULES : "${pkgs.glib-networking}/lib/gio/modules"
            --prefix LD_LIBRARY_PATH : "${pkgs.lib.makeLibraryPath [ pkgs.libayatana-appindicator ]}"
          )
        '';

        meta = {
          description = "Native Linux client for TIDAL";
          license = pkgs.lib.licenses.gpl3Only;
          mainProgram = "sone";
        };
      };

      devShells.${system}.default = pkgs.mkShell {
        nativeBuildInputs = with pkgs; [
          (rust-bin.stable.latest.default.override {
            extensions = [ "rust-src" "clippy" ];
          })
          cargo-tauri
          nodejs
          pkg-config
          gobject-introspection
        ];
        buildInputs = libraries;
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath libraries;
        shellHook = ''
          export GIO_MODULE_DIR="${pkgs.glib-networking}/lib/gio/modules"
          export GST_PLUGIN_PATH="${pkgs.lib.makeSearchPath "lib/gstreamer-1.0" gstDeps}"
        '';
      };
    };
}
