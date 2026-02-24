# Changelog

## [Unreleased]

### Breaking Changes

- **Removed `api_key` field from `LightClientConfig`.** The API key is now part of the `photon_url`:
  ```rust
  // Before
  LightClientConfig::new(
      "https://api.devnet.solana.com".to_string(),
      Some("https://photon.helius.com".to_string()),
      Some("YOUR_KEY".to_string()),
  )

  // After
  LightClientConfig::new(
      "https://api.devnet.solana.com".to_string(),
      Some("https://photon.helius.com?api-key=YOUR_KEY".to_string()),
  )
  ```

- **`LightClientConfig::new` takes 2 parameters instead of 3** (`url`, `photon_url`).

- **`LightClientConfig::devnet` takes 1 parameter instead of 2** (`photon_url`).

- **`PhotonIndexer::new` takes 1 parameter instead of 2** (`url`).

- **`LightClient::add_indexer` takes 1 parameter instead of 2** (`url`).

## [0.22.0] - 2026-02-17

### Fixed

- Fixed mint validation to apply to all token accounts, not just compressible ones. (#2251)
- Enforced canonical bump in ATA verification. (#2249)

### Changed

- Refactored `AccountInterface` to use photon v2 types and simplified `ColdContext`. (#2274)

## [0.21.0] - 2026-02-10

### Changed

- Replaced photon-api client with [progenitor](https://github.com/oxidecomputer/progenitor)-generated client. (#2219)

### Fixed

- Fixed tree infos v2 helpers. (#2244)

### Added

- Added compressed mint support via photon API. (#2198)
