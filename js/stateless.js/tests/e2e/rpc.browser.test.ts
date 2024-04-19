import { describe, it, assert, beforeAll, afterAll } from 'vitest';
import { Browser, Page, chromium } from 'playwright';
import {
    compressLamports,
    createRpc,
    defaultTestStateTreeAccounts,
    getTestKeypair,
    initSolOmnibusAccount,
    newAccountWithLamports,
} from '../../src';

describe('RPC in browser', () => {
    const { merkleTree } = defaultTestStateTreeAccounts();

    let browser: Browser;
    let page: Page;

    beforeAll(async () => {
        const rpc = createRpc();
        const payer = await newAccountWithLamports(rpc, 1000005000, 100);
        const initAuthority = await newAccountWithLamports(rpc, 1e9);

        browser = await chromium.launch();
        page = await browser.newPage();

        await initSolOmnibusAccount(rpc, initAuthority, initAuthority);
        await compressLamports(rpc, payer, 1e9, payer.publicKey, merkleTree);
    });

    it('getCompressedAccountsByOwner', async () => {
        const result = await page.evaluate(async () => {
            const rpc = createRpc();
            console.log('browser? rpc: ', rpc);
            // return 'hello world!';
            // const payer = getTestKeypair(100);
            // const compressedAccounts = await rpc.getCompressedAccountsByOwner(
            //     payer.publicKey,
            // );
            // if (!compressedAccounts)
            //     throw new Error('No compressed accounts found');
            // return compressedAccounts;
        });
        console.log('browser? result: ', result);
        // assert.equal(result.length, 1);
    });

    afterAll(async () => {
        await browser.close();
    });
});
