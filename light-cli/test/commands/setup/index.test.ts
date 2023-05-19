import {expect, test} from '@oclif/test'

describe('setup', () => {
  test
  .stdout()
  .command(['setup'])
  .it('runs setup cmd', ctx => {
    expect(ctx.stdout).to.contain('Setup completed successfully.')
  })
})
