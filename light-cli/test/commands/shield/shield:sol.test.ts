import { expect, test } from "@oclif/test";

describe("shield:sol", () => {
    test
    .stdout()
    .command([
      'shield:sol',
      '2.3',
    ])
    .it("Shielding 2.3 SOL", async (ctx) => {
      expect(ctx.stdout).to.contain('Successfully shielded 2.3 SOL ✔');
    })

    test
    .stdout()
    .command([
      'shield:sol',
      '123456789',
      '-d'
    ])
    .it("Shielding 123456789 LAMPORTS", async (ctx) => {
      expect(ctx.stdout).to.contain('Successfully shielded 0.123456789 SOL ✔');
    })

    test
    .stdout()
    .stderr()
    .command([
      'shield:sol', 
      '2222222222222222222222222222222222222222', 
    ])
    .exit(2)
    .it("Should fail shield of unsufficient SOL amount")

    test
    .stdout()
    .stderr()
    .command([
      'shield:sol', 
      '0.5', 
      '--recipient=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbcFAIL'
    ])
    .exit(2)
    .it("Should fail shield SOL to an invalid shielded recipient address")
})
