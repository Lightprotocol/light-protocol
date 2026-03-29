@unit
Feature: Solana Kit adapter
  The kit module converts legacy web3.js instructions to
  Solana Kit compatible instruction objects.

  Scenario: Convert legacy instructions to kit instructions
    Given a legacy create-ATA instruction
    When I convert it to kit instructions
    Then the result is a list of 1 kit instruction object

  Scenario: Wrap canonical builders for kit consumers
    Given random keypairs for "payer", "owner", and "mint"
    When I call the kit createAtaInstructions builder
    Then the result is a list of 1 kit instruction

  Scenario: Transfer and plan builders are exported
    Then createTransferInstructions from kit is a function
    And createTransferInstructionPlan from kit is a function
