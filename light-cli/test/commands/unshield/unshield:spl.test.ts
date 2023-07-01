import { expect, test } from "@oclif/test";

describe("unshield:spl", () => {
    test
    .stdout()
    .command([
        'unshield:spl', 
        '0.5', 
        'USDC', 
        'E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc'
    ])
    .it("Unshielding 0.5 SPL:USDC", async (ctx) => {
      expect(ctx.stdout).to.contain('Successfully unshielded 0.5 USDC ✔');
    })

    test
    .stdout()
    .stderr()
    .command([
        'unshield:spl', 
        '0.5', 
        'USDC', 
        'E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbcFAIL'
    ])
    .exit(2)
    .it("Should fail unshield to an invalid SPL recipient address")

    test
    .stdout()
    .stderr()
    .command([
        'unshield', 
        '55', 
        'USDC', 
        'E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc'
    ])
    .exit(2)
    .it("Should fail unshield of unsufficient SPL token amount")

    test
    .stdout()
    .command([
        'unshield', 
        '0.5', 
        'LFG', 
        'E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc'
    ])
    .exit(2)
    .it("Should fail unshield of unregistered SPL token")
})