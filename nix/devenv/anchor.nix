{ lib
, stdenv
, fetchurl
, autoPatchelfHook
, zlib
, openssl
, glibc
}:

let
  version = "0.31.1";

  sources = {
    x86_64-linux = {
      url = "https://github.com/coral-xyz/anchor/releases/download/v${version}/anchor-${version}-x86_64-unknown-linux-gnu";
      hash = "sha256-Xl+PwPdfLD3FzOsIKn9zXQm+IgdUApH/rTcOtbLclZs=";
    };
    x86_64-darwin = {
      url = "https://github.com/coral-xyz/anchor/releases/download/v${version}/anchor-${version}-x86_64-apple-darwin";
      hash = "sha256-MwGcRwS2x4toDyth5yJEiXKsZ6vokBnyCdCSAGhPTLs=";
    };
    aarch64-darwin = {
      url = "https://github.com/coral-xyz/anchor/releases/download/v${version}/anchor-${version}-aarch64-apple-darwin";
      hash = "sha256-ljxesAeqMwTXDV4H+Ng5AqHLiQLv7l3aRHzlSLk3lJQ=";
    };
  };

  platform = stdenv.hostPlatform.system;
  src = fetchurl {
    inherit (sources.${platform}) url hash;
  };

in stdenv.mkDerivation {
  pname = "anchor";
  inherit version src;

  dontUnpack = true;

  nativeBuildInputs = lib.optionals stdenv.isLinux [
    autoPatchelfHook
  ];

  buildInputs = lib.optionals stdenv.isLinux [
    zlib
    openssl
    stdenv.cc.cc.lib
  ];

  dontConfigure = true;
  dontBuild = true;

  installPhase = ''
    runHook preInstall
    mkdir -p $out/bin
    cp $src $out/bin/anchor
    chmod +x $out/bin/anchor
    runHook postInstall
  '';

  meta = with lib; {
    description = "Anchor framework for Solana";
    homepage = "https://anchor-lang.com";
    license = licenses.asl20;
    platforms = builtins.attrNames sources;
    mainProgram = "anchor";
  };

  passthru = { inherit version; };
}
