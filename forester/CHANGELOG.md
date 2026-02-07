# Changelog

## [Unreleased]

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
