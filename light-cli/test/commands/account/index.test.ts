import {expect, test} from '@oclif/test';

describe('account', () => {
  test
  .stdout()
  .command(['account'])
  .it('runs account cmd', ({stdout}) => {
    expect(stdout).to.contain("TpqsASoGWfR96tVd6ePkN55S2VucK5gLjXJM2abywRU3darrKYkdYadyJsQ9vndp2khowVzuj5ZYduxxxrUun2e")
  }) 
})