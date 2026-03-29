@e2e
Feature: Approve and revoke delegation

  Scenario: Full approve then revoke cycle
    Given a fresh mint fixture
    And an owner with 4000 compressed tokens
    And a new delegate
    When the owner approves the delegate for 1500 tokens
    Then the delegate is set on the token account with amount 1500
    And no compute budget instructions were included
    When the owner revokes the delegation
    Then the delegate is cleared and delegated amount is 0
    And no compute budget instructions were included in revoke
