@unit
Feature: Light-token instruction builders
  Low-level instruction builders produce correct program IDs,
  discriminators, and account orderings without any RPC calls.

  Scenario: Create a canonical light-token ATA instruction
    Given random keypairs for "payer", "owner", and "mint"
    When I build a create-ATA instruction for "payer", "owner", and "mint"
    Then the instruction program ID is the light-token program
    And account key 0 is "owner"
    And account key 1 is "mint"
    And account key 2 is "payer"

  Scenario: Create a checked transfer instruction
    Given random keypairs for "source", "destination", "mint", "authority", and "payer"
    When I build a checked transfer instruction for 42 tokens with 9 decimals
    Then the instruction program ID is the light-token program
    And the instruction discriminator byte is 12
    And account key 0 is "source"
    And account key 2 is "destination"

  Scenario: Create approve, revoke, freeze, and thaw instructions
    Given random keypairs for "tokenAccount", "owner", "delegate", "mint", and "freezeAuthority"
    When I build approve, revoke, freeze, and thaw instructions
    Then the approve instruction targets the light-token program
    And the revoke instruction targets the light-token program
    And the freeze discriminator byte is 10
    And the thaw discriminator byte is 11
