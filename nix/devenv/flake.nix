{
  description = "Light Protocol development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        # Rust version from rust-toolchain.toml
        rustVersion = "1.90.0";
        rustToolchain = pkgs.rust-bin.stable.${rustVersion}.default.override {
          extensions = [ "clippy" "rust-src" ];
        };

        # Cargo wrapper that handles `+toolchain` syntax (for cargo-build-sbf compatibility)
        cargoWrapper = pkgs.writeShellScriptBin "cargo" ''
          if [[ "$1" == +solana ]]; then
            # Use platform-tools for SBF compilation
            shift
            PLATFORM_TOOLS="''${SBF_SDK_PATH:-$HOME/.cache/solana-platform-tools}/dependencies/platform-tools"
            if [ -x "$PLATFORM_TOOLS/rust/bin/cargo" ]; then
              # Set RUSTC so cargo uses platform-tools rustc (supports -Z flags)
              export RUSTC="$PLATFORM_TOOLS/rust/bin/rustc"
              export RUSTDOC="$PLATFORM_TOOLS/rust/bin/rustdoc"
              exec "$PLATFORM_TOOLS/rust/bin/cargo" "$@"
            else
              echo "Error: Solana platform-tools not found at $PLATFORM_TOOLS" >&2
              echo "Run cargo-build-sbf once to download platform-tools" >&2
              exit 1
            fi
          elif [[ "$1" == +* ]]; then
            toolchain="''${1#+}"
            shift
            exec ${pkgs.rustup}/bin/rustup run "$toolchain" cargo "$@"
          else
            exec ${rustToolchain}/bin/cargo "$@"
          fi
        '';

        # Import custom packages
        solana = pkgs.callPackage ./solana.nix { };
        anchor = pkgs.callPackage ./anchor.nix { };

        # Versions (keep in sync with scripts/devenv/versions.sh)
        photonCommit = "3dbfb8e6772779fc89c640b5b0823b95d1958efc";

      in {
        packages = {
          inherit solana anchor;
          default = solana;
        };

        devShells.default = pkgs.mkShell {
          name = "light";

          packages = [
            # Languages
            pkgs.go
            cargoWrapper  # Must be before rustToolchain to shadow cargo
            rustToolchain
            pkgs.rustup  # For `cargo +toolchain` handling
            pkgs.nodejs_22
            pkgs.pnpm

            # Tools
            pkgs.jq
            pkgs.redis
            pkgs.gnumake
            pkgs.pkg-config
            pkgs.openssl
            pkgs.starship

            # Solana ecosystem
            solana
            anchor
          ];

          shellHook = ''
            # Environment variables
            export REDIS_URL="redis://localhost:6379"
            export SBF_OUT_DIR="target/deploy"

            # Solana platform-tools: copy SDK to writable location for cargo-build-sbf
            SOLANA_TOOLS_DIR="$HOME/.cache/solana-platform-tools/${solana.version}"
            if [ ! -d "$SOLANA_TOOLS_DIR/sbf" ]; then
              echo "Setting up Solana platform-tools SDK..."
              mkdir -p "$SOLANA_TOOLS_DIR"
              cp -r ${solana}/bin/platform-tools-sdk/* "$SOLANA_TOOLS_DIR/"
              chmod -R u+w "$SOLANA_TOOLS_DIR"
            fi
            export SBF_SDK_PATH="$SOLANA_TOOLS_DIR/sbf"

            # Install nightly rustfmt via rustup (for `cargo +nightly fmt`)
            if ! rustup run nightly rustfmt --version &>/dev/null; then
              echo "Installing nightly toolchain for rustfmt..."
              rustup toolchain install nightly --component rustfmt
            fi

            # Photon indexer (installed via cargo)
            if ! command -v photon &>/dev/null; then
              echo "Installing Photon indexer..."
              RUSTFLAGS="-A dead-code" cargo install \
                --git https://github.com/helius-labs/photon.git \
                --rev ${photonCommit} \
                --locked
            fi

            # Gnark proving keys (use absolute path)
            KEYS_DIR="$(pwd)/prover/server/proving-keys"
            if [ ! -d "$KEYS_DIR" ] || [ -z "$(ls -A "$KEYS_DIR" 2>/dev/null)" ]; then
              echo "Downloading gnark proving keys..."
              (cd prover/server && go run . download --run-mode=forester-test --keys-dir="$KEYS_DIR" --max-retries=10)
            fi

            # Node dependencies
            if [ ! -d "node_modules" ] || [ -z "$(ls -A node_modules 2>/dev/null)" ]; then
              echo "Installing node dependencies..."
              pnpm install
            fi

            # Mark that we're in the devenv (for custom prompts)
            export LIGHT_DEVENV=1

            # Initialize starship prompt if available and shell is interactive
            if [[ $- == *i* ]] && command -v starship &>/dev/null; then
              eval "$(starship init bash 2>/dev/null || starship init zsh 2>/dev/null || true)"
            fi

            echo ""
            echo "Light Protocol devenv activated"
            echo "  Solana: ${solana.version}"
            echo "  Anchor: ${anchor.version}"
            echo "  Rust:   ${rustVersion}"
            echo ""
          '';

        };
      });
}
