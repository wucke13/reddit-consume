with import <nixpkgs> {};

stdenv.mkDerivation {
  name = "rust-env";

  buildInputs = [
    rustup pkgconfig

    openssl
  ];
}
