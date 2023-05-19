import {expect, test} from '@oclif/test';

describe('accept-utxos', () => {
    test
    .stdout()
    .command([
    'accept-utxos',
    '--token=SOL',
    '--all'
    ])
    .it('accept all SOL inbox UTXOs', (ctx: any) => {
    expect(ctx.stdout).to.contain("Accepted SOL inbox UTXOs successfully ✔")
    }) 

    test
    .stdout()
    .command([
    'accept-utxos',
    '--token=USDC',
    '--all'
    ])
    .it('accept all USDC inbox UTXOs', (ctx: any) => {
    expect(ctx.stdout).to.contain("Accepted USDC inbox UTXOs successfully ✔")
    }) 
})