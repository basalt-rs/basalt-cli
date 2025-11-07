{
  description = "Basalt CLI";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }: flake-utils.lib.eachDefaultSystem (system: let
    pkgs = nixpkgs.legacyPackages.${system};
    rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
  in {

    devShells.default = pkgs.mkShell {
      buildInputs = with pkgs; [
        cargo rustc rustfmt clippy rust-analyzer glibc cargo-dist
      ];
      nativeBuildInputs = [ pkgs.pkg-config ];
      env.RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
    };

    packages.default = pkgs.rustPlatform.buildRustPackage rec {
      pname = "basalt-cli";
      version = "1.0.0";
      src = self;

      cargoLock = {
        lockFile = ./Cargo.lock;
      };

      buildInputs = with pkgs; [
        openssl
      ];
      nativeBuildInputs = with pkgs; [
        pkg-config
      ];

      # Set the RUST_SRC_PATH for the build if needed by your project
      # env = {
      #   RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
      # };

      # You can specify additional environment variables or commands for the build process
      # For example, if you need to pass features to cargo:
      # cargoBuildFlags = [ "--features" "my-feature" ];

      # Check phase (optional, but good practice for ensuring correctness)
      # doCheck = true; # Uncomment to run tests during the build

      # If your binary isn't named `basalt-cli`, you might need to adjust this
      # postInstall = ''
      #   mv $out/bin/my-other-binary $out/bin/basalt-cli
      # '';
      cargoBin = "basalt";

      meta = with pkgs.lib; {
        description = "Command line interface for provisioning Basalt competition servers";
        homepage = "https://basalt.rs";
        license = licenses.gpl3;
        mainProgram = "basalt";
        platforms = platforms.all; # Or specific platforms your project supports
      };
    };
  });
}
