{
  description = "Tauri dev shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f system);
    in
    {
      devShells = forAllSystems (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            config.allowUnfree = true;
			config.android_sdk.accept_license = true;
          };

          androidSdk = pkgs.androidenv.composeAndroidPackages {
            platformVersions = [ "35" "36" ];
            buildToolsVersions = [ "35.0.0" "36.0.0" ];
            includeNDK = true;
            includeEmulator = false;
          };
        in
        {
          default = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [
              pkg-config
              wrapGAppsHook4
              cargo
			  rustup
              cargo-tauri
              nodejs
			  pnpm
              rustc
              jdk17
              androidSdk.androidsdk
            ];

            buildInputs = with pkgs; [
              librsvg
              webkitgtk_4_1
            ];

            shellHook = ''
              export XDG_DATA_DIRS="$GSETTINGS_SCHEMAS_PATH"

              export JAVA_HOME="${pkgs.jdk17}"
              export ANDROID_HOME="${androidSdk.androidsdk}/libexec/android-sdk"
              export NDK_HOME="$ANDROID_HOME/ndk/$(ls -1 $ANDROID_HOME/ndk | head -1)"
            '';
          };
        });
    };
}
