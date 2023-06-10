import {expect, test} from '@oclif/test';

describe('airdrop', () => {
  test
  .stdout()
  .command(['airdrop', '1.5', 'ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k'])
  .it('airdrop 1.5 SOL to ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k', (ctx: any) => {
    expect(ctx.stdout).to.contain("Airdrop Successful ✔")
  }) 
  
  test
  .stdout()
  .command(['airdrop', '10','--token=USDC','E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc'])
  .it('airdrop 10 USDC to E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc', (ctx: any) => {
    expect(ctx.stdout).to.contain("Airdrop Successful ✔")
  }) 

  test
  .stdout()
  .stderr()
  .command(['airdrop', '1', 'E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbcsdf'])
  .exit(2)
  .it('airdrop 1 SOL to invalid address should fail!', (ctx: any) => {
    expect(ctx.stderr).to.contain("Error: Invalid public key input")
  }) 
})