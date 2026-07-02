{
  description = "Tauri dev shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, fenix }:
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

          # Rust toolchain + the Android std targets `tauri android build` needs
          # (mirrors the pattern used in the sibling 2cha-mobile project).
          rustToolchain = fenix.packages.${system}.combine [
            fenix.packages.${system}.stable.toolchain
            fenix.packages.${system}.targets.aarch64-linux-android.stable.rust-std
            fenix.packages.${system}.targets.armv7-linux-androideabi.stable.rust-std
            fenix.packages.${system}.targets.i686-linux-android.stable.rust-std
            fenix.packages.${system}.targets.x86_64-linux-android.stable.rust-std
          ];

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
              rustToolchain
              cargo-tauri
              nodejs
			  pnpm
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
              export ANDROID_SDK_ROOT="$ANDROID_HOME"
              export NDK_HOME="$ANDROID_HOME/ndk/$(ls -1 $ANDROID_HOME/ndk | head -1)"
              export ANDROID_NDK_HOME="$NDK_HOME"
              export ANDROID_NDK_ROOT="$NDK_HOME"
            '';
          };
        });
    };
}
