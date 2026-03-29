@e2e
Feature: Token transfers

  Scenario: Single-transaction transfer between light-token accounts
    Given a fresh mint fixture
    And a funded sender with 5000 compressed tokens
    And a new recipient
    When the sender transfers 2000 tokens to the recipient
    Then the recipient ATA balance is 2000
    And the sender ATA balance is 3000

  Scenario: Transfer to an SPL ATA recipient
    Given a fresh mint fixture
    And a funded sender with 3000 compressed tokens
    And a funded recipient with SOL
    When the sender transfers 1250 tokens to the recipient SPL ATA
    Then the recipient SPL ATA balance is 1250

  Scenario: Insufficient funds error propagates from on-chain
    Given a fresh mint fixture
    And a sender with compressed mints of 500, 300, and 200 tokens
    And a new recipient
    When the sender attempts to transfer 600 tokens
    Then the transaction fails with "custom program error"

  Scenario: Zero-amount transfer preserves balance
    Given a fresh mint fixture
    And a funded sender with 500 compressed tokens
    And a new recipient
    When the sender transfers 0 tokens to the recipient
    Then the sender ATA balance is 500

  Scenario: Recipient compressed balance is not loaded during transfer
    Given a fresh mint fixture
    And a funded sender with 400 compressed tokens
    And a funded recipient with 300 compressed tokens
    When the sender transfers 200 tokens to the recipient
    Then the recipient hot balance is 200
    And the recipient still has 300 in compressed accounts
    And the recipient total ATA amount is 500 with 300 compressed

  Scenario: Delegated transfer after approval
    Given a fresh mint fixture
    And an owner with 500 compressed tokens
    And a delegate approved for 300 tokens
    When the delegate transfers 250 tokens to a new recipient
    Then the recipient ATA balance is 250
    And the owner ATA balance is 250
