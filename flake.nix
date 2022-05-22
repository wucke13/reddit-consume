{
  inputs = {
    utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    naersk = {
      url = "github:nmattia/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
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
            targets.x86_64-unknown-linux-gnu.stable.rust-std
            targets.armv7a-none-eabi.stable.rust-std # TODO check this
          ];
        naersk-lib = (naersk.lib.${system}.override {
          cargo = rust-toolchain;
          rustc = rust-toolchain;
        });
      in
      rec {
        packages.reddit-consume = naersk-lib.buildPackage {
          src = ./.;
          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = with pkgs; [ mpv openssl ];
        };
        defaultPackage = packages.reddit-consume;

        apps.reddit-consume = utils.lib.mkApp { drv = packages.reddit-consume; name = "reddit-consume"; };
        defaultApp = apps.reddit-consume;

        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            rust-toolchain
            gcc
          ] ++ packages.reddit-consume.nativeBuildInputs;
          buildInputs = packages.reddit-consume.buildInputs;
        };
      });
}

