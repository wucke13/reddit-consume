{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }@inputs:
    flake-utils.lib.eachSystem [ "aarch64-linux" "i686-linux" "x86_64-linux" ] (system:
      let
        pkgs = nixpkgs.legacyPackages."${system}";
        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        name = cargoToml.package.name;
        version = cargoToml.package.version;
      in
      rec {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = name;
          inherit version;
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          nativeBuildInputs = with pkgs; [ pkg-config makeWrapper ];
          buildInputs = [ pkgs.openssl ];
          postInstall = ''
            wrapProgram $out/bin/${name} --suffix PATH : ${with pkgs; lib.makeBinPath [ mpv yt-dlp ]}
          '';
        };

        apps.default = flake-utils.lib.mkApp { inherit name; drv = packages.default; };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ packages.default ];
          nativeBuildInputs = with pkgs; [
            mpv
            yt-dlp

            clippy
            rustfmt
          ];
        };
      });
}

