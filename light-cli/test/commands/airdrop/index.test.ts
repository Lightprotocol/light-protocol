import {expect, test} from '@oclif/test';

describe('airdrop', () => {
  test
  .stdout()
  .command(['airdrop', '1.5', 'ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k'])
  .it('runs airdrop cmd', (ctx: any) => {
    expect(ctx.stdout).to.contain("\nAirdrop Successful \x1b[32m✔\x1b[0m")
  }) 
})