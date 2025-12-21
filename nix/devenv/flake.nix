{
  description = "Light Protocol development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        # Import custom packages
        solana = pkgs.callPackage ./solana.nix { };
        anchor = pkgs.callPackage ./anchor.nix { };

        # Smart build-sbf wrapper that uses per-program target dirs to avoid cache invalidation
        buildSbfWrapper = pkgs.writeShellScriptBin "build-sbf" ''
          set -e
          REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

          build_program() {
            local prog="$1"; shift
            local name=$(basename "$prog")
            echo "==> $prog"
            CARGO_TARGET_DIR="$REPO_ROOT/target-$name" cargo build-sbf --manifest-path "$prog/Cargo.toml" "$@"
          }

          if [ -f "Cargo.toml" ] && grep -q '^\[workspace\]' "Cargo.toml" 2>/dev/null; then
            echo "Building programs with separate target directories..."
            for prog in programs/*/; do
              [ -f "$prog/Cargo.toml" ] && build_program "$prog" "$@"
            done
          else
            name=$(basename "$PWD")
            export CARGO_TARGET_DIR="$REPO_ROOT/target-$name"
            exec cargo build-sbf "$@"
          fi
        '';

        # Smart test-sbf wrapper
        testSbfWrapper = pkgs.writeShellScriptBin "test-sbf" ''
          set -e
          REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

          test_program() {
            local prog="$1"; shift
            local name=$(basename "$prog")
            echo "==> $prog"
            CARGO_TARGET_DIR="$REPO_ROOT/target-$name" cargo test-sbf --manifest-path "$prog/Cargo.toml" "$@"
          }

          if [ -f "Cargo.toml" ] && grep -q '^\[workspace\]' "Cargo.toml" 2>/dev/null; then
            echo "Testing programs with separate target directories..."
            for prog in programs/*/; do
              [ -f "$prog/Cargo.toml" ] && test_program "$prog" "$@"
            done
          else
            name=$(basename "$PWD")
            export CARGO_TARGET_DIR="$REPO_ROOT/target-$name"
            exec cargo test-sbf "$@"
          fi
        '';

        # Versions (keep in sync with scripts/devenv/versions.sh)
        rustVersion = "1.90.0";
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
            pkgs.rustup
            pkgs.nodejs_22
            pkgs.pnpm

            # Tools
            pkgs.jq
            pkgs.redis
            pkgs.gnumake
            pkgs.pkg-config
            pkgs.openssl
            pkgs.starship
            pkgs.sccache  # Compile cache (helps with different feature combinations)

            # Solana ecosystem
            solana
            anchor
            buildSbfWrapper  # Smart wrapper: build-sbf
            testSbfWrapper   # Smart wrapper: test-sbf
          ];

          shellHook = ''
            # Environment variables
            export REDIS_URL="redis://localhost:6379"
            export SBF_OUT_DIR="target/deploy"

            # sccache is available but NOT enabled by default
            # It helps with different feature combinations but breaks cargo's incremental cache
            # Enable manually for CI: export RUSTC_WRAPPER=sccache
            export SCCACHE_DIR="$HOME/.cache/sccache"

            # Solana platform-tools: copy SDK to writable location for cargo-build-sbf
            SOLANA_TOOLS_DIR="$HOME/.cache/solana-platform-tools/${solana.version}"
            if [ ! -d "$SOLANA_TOOLS_DIR/sbf" ]; then
              echo "Setting up Solana platform-tools SDK..."
              mkdir -p "$SOLANA_TOOLS_DIR"
              cp -r ${solana}/bin/platform-tools-sdk/* "$SOLANA_TOOLS_DIR/"
              chmod -R u+w "$SOLANA_TOOLS_DIR"
            fi
            export SBF_SDK_PATH="$SOLANA_TOOLS_DIR/sbf"

            # Rust toolchain (managed by rustup, not nix)
            if ! rustup show active-toolchain 2>/dev/null | grep -q "${rustVersion}"; then
              echo "Installing Rust ${rustVersion}..."
              rustup install ${rustVersion}
              rustup default ${rustVersion}
              rustup component add clippy
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
