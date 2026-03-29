@e2e
Feature: Load compressed balances into hot account

  Scenario: getAta exposes the biggest compressed balance
    Given a fresh mint fixture
    And an owner with compressed mints of 400, 300, and 200 tokens
    When I read the owner ATA
    Then the ATA amount is 400
    And the ATA compressed amount is 400
    And the ATA requires load
    And there are 2 ignored compressed accounts totaling 500

  Scenario: Load one compressed balance per call
    Given a fresh mint fixture
    And an owner with compressed mints of 500, 300, and 200 tokens
    When I load the first compressed balance
    Then the hot balance is 500
    And compressed accounts are 300 and 200
    When I load the next compressed balance
    Then the hot balance is 800
    And compressed accounts are 200
