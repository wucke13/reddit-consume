{
  inputs = {
    utils.url = "github:numtide/flake-utils";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
    naersk.url = "github:nix-community/naersk";
    naersk.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, utils, fenix, naersk, ... }@inputs:
    utils.lib.eachSystem [ "aarch64-linux" "i686-linux" "x86_64-linux" ] (system:
      let
        pkgs = nixpkgs.legacyPackages."${system}";
        rust-toolchain = with fenix.packages.${system};
          combine [
            stable.rustc
            stable.cargo
            stable.clippy
            stable.rustfmt
            targets.aarch64-unknown-linux-gnu.stable.rust-std
            targets.i686-unknown-linux-musl.stable.rust-std
            targets.x86_64-unknown-linux-musl.stable.rust-std
          ];
        naersk-lib = (naersk.lib.${system}.override {
          cargo = rust-toolchain;
          rustc = rust-toolchain;
        });
        name = "reddit-consume";
      in
      rec {
        packages.default = naersk-lib.buildPackage {
          inherit name;
          src = ./.;
          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.openssl ];
          propagatedBuildInputs = [ pkgs.mpv ];
        };

        apps.default = utils.lib.mkApp { inherit name; drv = packages.default; };

        devShells.default = pkgs.mkShell { inputsFrom = [ packages.default ]; };
      });
}

