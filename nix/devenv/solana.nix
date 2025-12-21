{ lib
, stdenv
, fetchurl
, autoPatchelfHook
, zlib
, openssl
, udev
, libclang
, llvmPackages
}:

let
  version = "2.2.15";

  sources = {
    x86_64-linux = {
      url = "https://github.com/anza-xyz/agave/releases/download/v${version}/solana-release-x86_64-unknown-linux-gnu.tar.bz2";
      hash = "sha256-KfOtGQo9sjB+ZiH0Q0qSXBsQJe8I1Ydr+lqDKanopX4=";
    };
    x86_64-darwin = {
      url = "https://github.com/anza-xyz/agave/releases/download/v${version}/solana-release-x86_64-apple-darwin.tar.bz2";
      hash = "sha256-uCnX3MkGf3oxKNna8U7+GA4ux9E8Mcsb1WDpkdZc8sQ=";
    };
    aarch64-darwin = {
      url = "https://github.com/anza-xyz/agave/releases/download/v${version}/solana-release-aarch64-apple-darwin.tar.bz2";
      hash = "sha256-4ycCeVg/EenfWwLO0erK2ryTQ4VSXNWk3nw+W8WQjX8=";
    };
  };

  platform = stdenv.hostPlatform.system;
  src = fetchurl {
    inherit (sources.${platform}) url hash;
  };

in stdenv.mkDerivation {
  pname = "solana-cli";
  inherit version src;

  sourceRoot = ".";

  nativeBuildInputs = lib.optionals stdenv.isLinux [
    autoPatchelfHook
  ];

  buildInputs = lib.optionals stdenv.isLinux [
    zlib
    openssl
    stdenv.cc.cc.lib
  ] ++ lib.optionals (stdenv.isLinux && udev != null) [
    udev
  ];

  dontConfigure = true;
  dontBuild = true;

  installPhase = ''
    runHook preInstall
    mkdir -p $out/bin
    cp -r solana-release/bin/* $out/bin/
    runHook postInstall
  '';

  meta = with lib; {
    description = "Solana CLI tools";
    homepage = "https://solana.com";
    license = licenses.asl20;
    platforms = builtins.attrNames sources;
    mainProgram = "solana";
  };

  passthru = { inherit version; };
}
