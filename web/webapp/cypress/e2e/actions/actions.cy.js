"use client";
/// <reference types="cypress" />
Cypress.on("uncaught:exception", (err, runnable) => {
  // returning false here prevents Cypress from
  // failing the test
  return false;
});

describe("light web-app", () => {
  beforeEach(() => {
    cy.wait(15000);
    cy.visit("http://127.0.0.1:3000/");
    // wait for login
    cy.contains("My assets", { timeout: 10000 }).should("be.visible");
  });

  let startBalance = 0;
  const shieldAmount0 = "0.01";

  it("should store the previous balance", () => {
    // Store the previous balance
    cy.get('td:contains("SOL")')
      .next()
      .invoke("text")
      .then((text) => {
        startBalance = parseFloat(text);
        console.log("STARTBALANCE", startBalance);
      });
  });

  it("should display Shield&Send Button and open and close it", () => {
    cy.get('button:contains("Shield & Send")').click();
    // Check that the modal is visible
    cy.get("form").should("be.visible");
    // Click outside the modal to close it
    cy.get("body").click(0, 0);
    // Check that the modal is not visible
    cy.wait(1000);
    cy.get("form", { timeout: 0 }).should("not.exist");
  });

  it("should fill form and shield", () => {
    // Open modal
    cy.get('button:contains("Shield & Send")').click();
    cy.get("form").should("be.visible");

    // Check that the token and amount inputs are visible
    cy.get('[data-testid="amount-input"]').should("be.visible");
    cy.get('[data-testid="token-dropdown"]').should("be.visible");

    // type amount
    cy.get('[data-testid="amount-input"]').type(shieldAmount0);

    // Select token
    cy.get('[data-testid="token-dropdown"]').click();
    cy.get('[data-testid="token-option-SOL"]').click();

    cy.get('button:contains("Shield now")').should("be.visible");
    cy.get('button:contains("Shield now")').click();
    cy.get('button:contains("Shield now")').should("be.disabled"); // loading

    cy.contains("Shielding SOL", { timeout: 20000 }).should("be.visible");
    cy.contains("Shield successful", { timeout: 40000 }).should("be.visible");
    cy.wait(2000);
    cy.get('[data-testid="shield-send-modal"]', { timeout: 0 }).should(
      "not.exist"
    );
  });
  it("should update balance by the amount that was just shielded", () => {
    // Check that the balance has been updated
    cy.get('td:contains("SOL")')
      .next()
      .invoke("text")
      .then((text) => {
        expect(parseFloat(text)).to.be.closeTo(
          startBalance + parseFloat(shieldAmount0),
          0.2
        );
      });
  });
  it("should display the transaction card of the recent shield as topmost transaction", () => {
    cy.get("[data-testid='TransactionCard']").should("have.length.at.least", 1);

    cy.get("[data-testid='TransactionCard']").first().contains("shield");
  });

  const sendAmount0 = "0.001";

  it("should fill form and send", () => {
    // Open modal
    cy.get('button:contains("Shield & Send")').click();
    cy.get("form").should("be.visible");

    // Switch to sendform
    cy.get('[data-testid="shield-send-control"]').contains("Send").click();

    cy.get('[data-testid="send-form"]').should("be.visible");

    // Check that the token and amount inputs are visible
    cy.get('[data-testid="amount-input"]').should("be.visible");
    cy.get('[data-testid="token-dropdown"]').should("be.visible");

    // type amount
    cy.get('[data-testid="amount-input"]').type(sendAmount0);

    // type recipient
    const rec = "3D4FEitQszTU5yFtEc91JddoGAtvCeqDj6om7Wj6VEqK";
    cy.get('[data-testid="recipient-input"]').type(rec, { delay: 40 });

    // Select token usdc -> sol
    cy.get('[data-testid="token-dropdown"]').click();
    cy.get('[data-testid="token-option-USDC"]').click();
    cy.get('[data-testid="token-dropdown"]').click();
    cy.get('[data-testid="token-option-SOL"]').click();

    cy.get('button:contains("Send now")').should("be.visible");
    cy.get('button:contains("Send now")').click();
    cy.get('button:contains("Send now")').should("be.disabled"); // loading

    cy.contains("Unshielding SOL", { timeout: 20000 }).should("be.visible");
    cy.contains("Unshield successful", { timeout: 40000 }).should("be.visible");
    cy.wait(2000);
    cy.get('[data-testid="shield-send-modal"]', { timeout: 0 }).should(
      "not.exist"
    );
  });

  it("should update balance by the amount that was just sent", () => {
    // Check that the balance has been updated
    cy.get('td:contains("SOL")')
      .next()
      .invoke("text")
      .then((text) => {
        expect(parseFloat(text)).to.be.closeTo(
          startBalance + parseFloat(shieldAmount0) - parseFloat(sendAmount0),
          0.5
        );
      });
  });

  it("should display the transaction card of the recent send as topmost transaction", () => {
    cy.get("[data-testid='TransactionCard']").should("have.length.at.least", 1); // TODO: make a dynamic count
    cy.get("[data-testid='TransactionCard']").first().contains("unshield");
  });
});
