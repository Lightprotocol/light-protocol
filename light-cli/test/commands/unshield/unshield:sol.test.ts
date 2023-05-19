import { expect, test } from "@oclif/test";

describe("unshield:sol", () => {
    test
    .stdout()
    .command([
        'unshield:sol', 
        '0.2', 
        'E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc'
    ])
    .it("Unshielding 0.2 SOL", async (ctx) => {
      expect(ctx.stdout).to.contain('Successfully unshielded 0.2 SOL ✔');
    })

    test
    .stdout()
    .stderr()
    .command([
        'unshield:sol', 
        '0.5', 
        'E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbcFAIL'
    ])
    .exit(2)
    .it("Should fail unshield to an invalid SOL recipient address")

    test
    .stdout()
    .stderr()
    .command([
        'unshield:sol', 
        '300', 
        'E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc'
    ])
    .exit(2)
    .it("Should fail unshield of unsufficient SOL amount")
})
