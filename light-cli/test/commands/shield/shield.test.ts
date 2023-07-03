import { expect, test } from "@oclif/test";

describe("shield SOL & SPL separately with the main command", () => {
    test
    .stdout()
    .command([
      'shield', 
      '--amount-sol=7',
    ])
    .it("Shielding 7 SOL", async (ctx) => {
      expect(ctx.stdout).to.contain('Successfully shielded 7 SOL ✔');
    })

    test
    .stdout()
    .command([
      'shield',
      '--amount-spl=9',
      '--token=USDC',
    ])
    .it("Shielding 9 SPL:USDC", async (ctx) => {
      expect(ctx.stdout).to.contain('Successfully shielded 9 USDC ✔');
    })

    test
    .stdout()
    .stderr()
    .command([
      'shield', 
      '--amount-sol=22222222222222222222222222222222',
    ])
    .exit(2)
    .it("Should fail shield of unsufficient SOL amount")

    test
    .stdout()
    .stderr()
    .command([
      'shield',
      '--amount-spl=5555555555555555555555555555555',
      '--token=USDC',
    ])
    .exit(2)
    .it("Should fail shield of unsufficient SPL amount")

    test
    .stdout()
    .stderr()
    .command([
      'shield', 
      '--amount-sol=0.2', 
      '--recipient=TpqsASoGWfR96tVd6ePkN55S2VucK5gLjXJM2abywRU3darrKYkdYadyJsQ9vndp2khowVzuj5ZYduxxxrUun2eFAIL',
    ])
    .exit(2)
    .it("Should fail shield to invalid shielded recipient address")

    test
    .stdout()
    .stderr()
    .command([
      'shield', 
      '--amount-spl=3', 
      '--token=LFG',
    ])
    .exit(2)
    .it("Should fail shield of unregistered SPL token")
})

describe("shield SOL & SPL at the same time with the main command", () => {
  test
  .stdout()
  .command([
    'shield', 
    '--amount-sol=5', 
    '--amount-spl=1', 
    '--token=USDC',
  ])
  .it("Shielding 5 SOL & 1 SPL:USDC at the same time with the main cli", async (ctx) => {
    expect(ctx.stdout).to.contain('Successfully shielded 5 SOL & 1 USDC ✔');
  });

  test
  .stdout()
  .stderr()
  .command([
    'shield', 
    '--amount-sol=222222222222222222222222222222222', 
    '--amount-spl=3', 
    '--token=USDC',
  ])
  .exit(2)
  .it("Should fail shield of unsufficient SOL amount")

  test
  .stdout()
  .stderr()
  .command([
    'shield', 
    '--amount-sol=0.2', 
    '--amount-spl=33333333333333333333333333333333', 
    '--token=USDC',
  ])
  .exit(2)
  .it("Should fail shield of unsufficient SPL amount")

  test
  .stdout()
  .stderr()
  .command([
    'shield', 
    '--amount-sol=0.2', 
    '--amount-spl=33', 
    '--token=USDC',
    '--recipient=TpqsASoGWfR96tVd6ePkN55S2VucK5gLjXJM2abywRU3darrKYkdYadyJsQ9vndp2khowVzuj5ZYduxxxrUun2eFAIL',
  ])
  .exit(2)
  .it("Should fail shield to invalid shielded recipient address")

  test
  .stdout()
  .stderr()
  .command([
    'shield', 
    '--amount-sol=0.2', 
    '--amount-spl=3', 
    '--token=LFG',
  ])
  .exit(2)
  .it("Should fail shield of unregistered SPL token")

});