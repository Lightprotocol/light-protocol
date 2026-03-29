@unit
Feature: Public API surface
  The root package export exposes address derivation, instruction
  builders, discriminators, and error types.

  Scenario: Derive the canonical light-token ATA address
    Given random keypairs for "owner" and "mint"
    When I derive the ATA address for "owner" and "mint"
    Then it matches the low-level getAssociatedTokenAddress result

  Scenario: Build one canonical ATA instruction
    Given random keypairs for "payer", "owner", and "mint"
    When I build an ATA instruction list for "payer", "owner", and "mint"
    Then the result is a list of 1 instruction
    And the first instruction program ID is the light-token program

  Scenario: Raw freeze and thaw discriminators
    Given random keypairs for "tokenAccount", "mint", and "freezeAuthority"
    When I build raw freeze and thaw instructions
    Then the freeze discriminator byte is 10
    And the thaw discriminator byte is 11

  Scenario: Single-transaction error is clear
    When I create a MultiTransactionNotSupportedError for "createLoadInstructions" with batch count 2
    Then the error name is "MultiTransactionNotSupportedError"
    And the error message contains "single-transaction"
    And the error message contains "createLoadInstructions"

  Scenario: Canonical transfer builder is exported
    Then createTransferInstructions is a function
