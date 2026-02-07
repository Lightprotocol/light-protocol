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
