# Changelog

## [Unreleased]

### Added

- **Graceful shutdown signaling** via `watch::channel`. Shutdown requests are now race-free regardless of when the run loop subscribes.
- **Panic isolation for `process_epoch`.** A panicking epoch no longer kills the run loop; the panic message is logged and processing continues.

### Fixed

- **`bigint_to_u8_32` now rejects negative `BigInt` inputs** (`light-prover-client`). Previously, negative inputs were silently converted to `[u8; 32]` using only the magnitude bytes, producing wrong-sign output that would cause silent proof-input corruption.
- **`pathIndex` widened from `u32` to `u64`** on both the Rust client and the Go prover server. The Gnark circuit already constrained by tree height (up to 40 bits for v2 address trees); only the JSON marshalling and runtime struct types were artificially narrow. This prevents proof generation failures once a v2 address tree exceeds ~4.3 billion entries.

### Breaking Changes

- **Removed `--photon-api-key` CLI arg and `PHOTON_API_KEY` env var.** The API key should now be included in `--indexer-url` as a query parameter:
  ```bash
  # Before
  --indexer-url https://photon.helius.com --photon-api-key YOUR_KEY

  # After
  --indexer-url "https://photon.helius.com?api-key=YOUR_KEY"
  ```

- **Removed `photon_api_key` field from `ExternalServicesConfig`.** The `indexer_url` field now carries the full URL including the API key.

- **Removed `ExternalServicesConfig::photon_url()` helper.** Use `indexer_url` directly instead.
