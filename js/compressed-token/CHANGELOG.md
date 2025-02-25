## [0.20.5-0.20.7] - 2025-02-24

### Changed

-   improved documentation and error messages.

## [0.20.4] - 2025-02-19

### Breaking Changes

-   `selectMinCompressedTokenAccountsForTransfer` and
    `selectSmartCompressedTokenAccountsForTransfer` now throw an error
    if not enough accounts are found. In most cases this is not a breaking
    change, because a proof request would fail anyway. This just makes the error
    message more informative.

### Added

-   `selectSmartCompressedTokenAccountsForTransfer` and
    `selectSmartCompressedTokenAccountsForTransferorPartial`

### Changed

-   `selectMinCompressedTokenAccountsForTransfer` and
    `selectMinCompressedTokenAccountsForTransferorPartial` now accept an optional
    `maxInputs` parameter, defaulting to 4.

### Security

-   N/A

For previous release notes, check:
https://www.zkcompression.com/release-notes/1.0.0-mainnet-beta
