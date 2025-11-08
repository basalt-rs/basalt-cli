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
        cargo rustc rustfmt clippy rust-analyzer glibc cargo-dist openssl
      ];
      nativeBuildInputs = [ pkgs.pkg-config ];
      env.RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
    };

    packages.basalt-cli = pkgs.rustPlatform.buildRustPackage rec {
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

      cargoBin = "basalt";

      meta = with pkgs.lib; {
        description = "Command line interface for provisioning Basalt competition servers";
        homepage = "https://basalt.rs";
        license = licenses.gpl3;
        mainProgram = "basalt";
        platforms = platforms.all; # Or specific platforms your project supports
      };
    };
    packages.default = self.packages.${system}.basalt-cli;
  });
}
