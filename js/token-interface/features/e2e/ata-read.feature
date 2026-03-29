@e2e
Feature: ATA creation and reads

  Scenario: Create and read back an ATA
    Given a fresh mint fixture
    And a new owner
    When the owner creates an ATA
    Then reading the ATA returns the correct address, owner, mint, and zero balance
