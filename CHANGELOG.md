# Changelog

## [Unreleased]

### Breaking Changes

#### `photon-api` crate

- **Simplified `Configuration` struct.** The API key is now embedded in the URL as a query parameter:
  ```rust
  // Before
  let mut config = Configuration::new();
  config.base_path = "https://photon.helius.com".to_string();
  config.api_key = Some(ApiKey { prefix: None, key: "YOUR_KEY".to_string() });

  // After
  let config = Configuration::new("https://photon.helius.com?api-key=YOUR_KEY".to_string());
  ```

- **Removed `Configuration::new_with_api_key`.** Use `Configuration::new` with the API key in the URL instead.

#### `light-client` crate

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

#### `forester-utils` crate

- **Removed `api_key` field and `.api_key()` builder method from `SolanaRpcPoolBuilder`.**

- **Removed `api_key` parameter from `SolanaConnectionManager::new`.**

#### `forester` crate

- **Removed `--photon-api-key` CLI arg and `PHOTON_API_KEY` env var.** The API key should now be included in `--indexer-url` as a query parameter:
  ```bash
  # Before
  --indexer-url https://photon.helius.com --photon-api-key YOUR_KEY

  # After
  --indexer-url "https://photon.helius.com?api-key=YOUR_KEY"
  ```

- **Removed `photon_api_key` field from `ExternalServicesConfig`.** The `indexer_url` field now carries the full URL including the API key.

- **Removed `ExternalServicesConfig::photon_url()` helper.** Use `indexer_url` directly instead.

### Added

- **`external/photon` git submodule.** The Photon OpenAPI spec (`external/photon/src/openapi/specs/api.yaml`) is now pulled in as a submodule and used at build time by `photon-api/build.rs` to generate client types via [progenitor](https://github.com/oxidecomputer/progenitor). This replaces the previously checked-in, manually maintained API models.

- **`submodules: true`** added to all GitHub Actions checkout steps to ensure the `external/photon` OpenAPI spec is available during CI builds.