with import <nixpkgs> {};

stdenv.mkDerivation {
  name = "rust-env";

  buildInputs = [
    zsh
    pkg-config rustup
    #musl.all
    #openssl.dev
  ];

  shellHook = ''
    export NIX_ENFORCE_PURITY=0
    export PKG_CONFIG_ALLOW_CROSS=1
    #export CC=musl-gcc
    #export LD=ld.musl-clang
    #export RUSTFLAGS="-C linker=$LD"
    exec zsh
  '';
}
