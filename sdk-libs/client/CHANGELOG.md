# Changelog

All notable changes to this package will be documented in this file.

## 2026-02-27

### Features

- Forester dashboard with compression improvements, pending state tracking, and eligibility checks. (#2310)

## 2026-02-17

### Features

- `AccountInterface` uses photon v2 types, `ColdContext` simplified. (#2274)

### Fixes

- `validate_mint()` validates mint for all token accounts, not just compressible. (#2251)
- Enforces canonical bump in ATA verification. (#2249)

## 2026-02-10

### Breaking Changes

- `LightClientConfig::new()` takes 2 parameters instead of 3. The API key is now part of `photon_url`.
  Before: `LightClientConfig::new(url, photon_url, Some(api_key))`
  After: `LightClientConfig::new(url, Some("https://photon.helius.com?api-key=YOUR_KEY"))`
  Migration: embed the API key as a query parameter in the photon URL. (#2219)

- `LightClientConfig::devnet()` takes 1 parameter instead of 2. (#2219)

- `PhotonIndexer::new()` takes 1 parameter instead of 2. (#2219)

- `LightClient::add_indexer()` takes 1 parameter instead of 2. (#2219)

### Features

- `compressed_mint` photon API support. (#2198)

### Fixes

- Tree infos v2 helpers fixed. (#2244)
