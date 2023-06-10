import {expect, test} from '@oclif/test'

describe('test-validator', () => {
  test
  .stdout()
  .command(['test-validator'])
  .it('runs test-validator cmd', ctx => {
    expect(ctx.stdout).to.contain("Setup tasks completed successfully ✔")
  })
})
