import {expect, test} from '@oclif/test';
import { initTestEnvIfNeeded } from '../../../src/utils/initTestEnv';
import { AUTHORITY_ONE, airdropShieldedMINTSpl, airdropShieldedSol, airdropSol } from '@lightprotocol/zk.js';
import { getConfig, readWalletFromFile } from '../../../src/utils/utils';
import { Connection } from '@solana/web3.js';

describe('accept-utxos', () => {
    before(async () => {
        await initTestEnvIfNeeded();
        const configWallet = readWalletFromFile();
        const config = getConfig();
        const connection = new Connection(config.rpcUrl, 'confirmed');
        await airdropSol({connection, amount: 10e9, recipientPublicKey: configWallet.publicKey});
        await airdropSol({connection, amount: 10e9, recipientPublicKey: AUTHORITY_ONE});
        await airdropShieldedSol({recipientPublicKey: "TpqsASoGWfR96tVd6ePkN55S2VucK5gLjXJM2abywRU3darrKYkdYadyJsQ9vndp2khowVzuj5ZYduxxxrUun2e", amount: 1})
        await airdropShieldedMINTSpl({recipientPublicKey: "TpqsASoGWfR96tVd6ePkN55S2VucK5gLjXJM2abywRU3darrKYkdYadyJsQ9vndp2khowVzuj5ZYduxxxrUun2e", amount: 1})
      })

    test
    .stdout({print: true})
    .command([
    'accept-utxos',
    '--token=SOL',
    '--all'
    ])
    .it('accept all SOL inbox UTXOs', (ctx: any) => {
    expect(ctx.stdout).to.contain("Accepted SOL inbox UTXOs successfully ✔")
    }) 

    test
    
    .stdout({print: true})
    .command([
    'accept-utxos',
    '--token=USDC',
    '--all'
    ])
    .it('accept all USDC inbox UTXOs', (ctx: any) => {
    expect(ctx.stdout).to.contain("Accepted USDC inbox UTXOs successfully ✔")
    }) 
})