import { expect, test } from "@oclif/test";

describe("unshield SOL & SPL separately with the main command", () => {
    test
    .stdout()
    .command([
        'unshield', 
        '--amount-sol=0.2', 
        '--recipient-sol=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc'
    ])
    .it("Unshielding 0.2 SOL", async (ctx) => {
      expect(ctx.stdout).to.contain('Successfully unshielded 0.2 SOL ✔');
    });

    test
    .stdout()
    .command([
        'unshield', 
        '--amount-spl=0.5', 
        '--token=USDC', 
        '--recipient-spl=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc'
    ])
    .it("Unshielding 0.5 SPL:USDC", async (ctx) => {
      expect(ctx.stdout).to.contain('Successfully unshielded 0.5 USDC ✔');
    });

    test
    .stdout()
    .stderr()
    .command([
        'unshield', 
        '--amount-sol=0.5', 
        '--recipient-sol=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbcFAIL'
    ])
    .exit(2)
    .it("Should fail unshield to an invalid SOL recipient address")

    test
    .stdout()
    .stderr()
    .command([
        'unshield', 
        '--amount-sol=30', 
        '--recipient-sol=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc'
    ])
    .exit(2)
    .it("Should fail unshield of unsufficient SOL amount")

    test
    .stdout()
    .stderr()
    .command([
        'unshield', 
        '--amount-spl=0.5', 
        '--token=USDC', 
        '--recipient-spl=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbcFAIL'
    ])
    .exit(2)
    .it("Should fail unshield to an invalid SPL recipient address")

    test
    .stdout()
    .stderr()
    .command([
        'unshield', 
        '--amount-spl=55', 
        '--token=USDC', 
        '--recipient-spl=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc'
    ])
    .exit(2)
    .it("Should fail unshield of unsufficient SPL token amount")

    test
    .stdout()
    .command([
        'unshield', 
        '--amount-spl=0.5', 
        '--token=LFG', 
        '--recipient-spl=E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc'
    ])
    .exit(2)
    .it("Should fail unshield of unregistered SPL token")
})