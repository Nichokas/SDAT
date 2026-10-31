{

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
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
      in
      {
        devShell = pkgs.mkShell (
          {
            GSETTINGS_SCHEMA_DIR = "./internal/settings";
            TONEARM_DEBUG = "1";

            buildInputs =
              with pkgs;
              [
                appstream
                cargo
                rustc
                clippy
                rustfmt
                rust-analyzer
                gtk4
                libadwaita
                openssl
                librsvg
                libsecret
                graphviz
                glib-networking
                gst_all_1.gstreamer
                gst_all_1.gst-plugins-base
                gst_all_1.gst-plugins-good
                gst_all_1.gst-plugins-bad
                pkg-config # Needed by the gtk4-rs / gstreamer-rs -sys crates
                sass
                cacert
              ]
              ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
                flatpak-builder
              ];
          }
          // pkgs.lib.optionalAttrs pkgs.stdenv.hostPlatform.isDarwin {
            # Only set GIO_EXTRA_MODULES on Darwin where the system gio
            # module path is unavailable. On Linux, leaving this unset lets
            # gio find the system glib-networking modules required for
            # HTTPS / TLS in souphttpsrc; setting it (even to empty) breaks
            # network streaming.
            GIO_EXTRA_MODULES = "${pkgs.glib-networking}/lib/gio/modules";
          }
        );

        packages.tonearm = pkgs.rustPlatform.buildRustPackage (finalAttrs: {
          pname = "tonearm";
          version = "1.5.1";
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;

          env = {
            TONEARM_COMMIT = if (self ? rev) then self.rev else "";
            TONEARM_VERSION = finalAttrs.version;
          };

          buildInputs = with pkgs; [
            glib-networking # TLS support for libsoup (HTTPS streaming)
            gst_all_1.gstreamer
            gst_all_1.gst-plugins-base
            gst_all_1.gst-plugins-good
            gst_all_1.gst-plugins-bad
            gtk4
            libadwaita
          ];
          doCheck = false;
          nativeBuildInputs = with pkgs; [
            pkg-config
            glib # glib-compile-schemas, used by build.rs
            copyDesktopItems
            makeWrapper
            wrapGAppsHook4
          ];

          desktopItems = [
            (pkgs.makeDesktopItem {
              name = "dev.dergs.Tonearm";
              exec = "tonearm %u";
              icon = "dev.dergs.Tonearm";
              comment = "Tonearm is a GTK client for TIDAL written in Rust.";
              desktopName = "Tonearm";
              mimeTypes = [
                "x-scheme-handler/tidal"
              ];
              categories = [
                "Audio"
                "AudioVideo"
                "Music"
                "GNOME"
                "GTK"
              ];
            })
          ];

          postInstall = ''
            wrapProgram $out/bin/tonearm \
              --prefix GST_PLUGIN_PATH : "$GST_PLUGIN_SYSTEM_PATH_1_0" \
              ''${gappsWrapperArgs[@]}
            install -Dm644 internal/icons/hicolor/scalable/apps/dev.dergs.Tonearm.svg -t $out/share/icons/hicolor/scalable/apps
            install -Dm644 internal/icons/hicolor/128x128/apps/dev.dergs.Tonearm.png -t $out/share/icons/hicolor/128x128/apps
            install -Dm644 internal/icons/hicolor/symbolic/apps/dev.dergs.Tonearm-symbolic.svg -t $out/share/icons/hicolor/symbolic/apps
            install -Dm644 internal/settings/dev.dergs.Tonearm.gschema.xml -t $out/share/glib-2.0/schemas
            glib-compile-schemas $out/share/glib-2.0/schemas
          '';

          meta = {
            description = "Tonearm is a GTK client for TIDAL written in Rust.";
            homepage = "https://codeberg.org/Dergs/Tonearm";
            license = pkgs.lib.licenses.gpl3Plus;
            maintainers = with pkgs.lib.maintainers; [
              drafolin
              nilathedragon
            ];
            mainProgram = "tonearm";
          };
        });

        packages.default = self.packages.${system}.tonearm;
      }
    );
}
