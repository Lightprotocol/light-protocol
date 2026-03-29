@e2e
Feature: Freeze and thaw token accounts

  Scenario: Freeze then thaw a hot account
    Given a fresh mint fixture with freeze authority
    And an owner with a created ATA and 2500 compressed tokens
    When the freeze authority freezes the account
    Then the account state is "Frozen"
    When the freeze authority thaws the account
    Then the account state is "Initialized"
