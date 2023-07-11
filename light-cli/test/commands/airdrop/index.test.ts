import {expect, test} from '@oclif/test';
import { initTestEnvIfNeeded } from '../../../src/utils/initTestEnv';

describe('airdrop', () => {
  before(async () => {
    await initTestEnvIfNeeded();
  })
  test
  .stdout()
  .command(['airdrop', '5', 'ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k'])
  .it('airdrop 5 SOL to ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k', (ctx: any) => {
    expect(ctx.stdout).to.contain("Airdrop Successful ✔")
  }) 
  
  test
  .stdout()
  .command([
    'airdrop',
    '10',
    '--token=USDC',
    'E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc'
  ])
  .it('airdrop 10 USDC to E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc', (ctx: any) => {
    expect(ctx.stdout).to.contain("Airdrop Successful ✔")
  })
})