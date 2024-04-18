import { describe, it, assert, beforeAll, afterAll } from 'vitest';
import { Browser, Page, chromium } from 'playwright';
import {
    Rpc,
    bn,
    compressLamports,
    createRpc,
    defaultTestStateTreeAccounts,
    getTestKeypair,
    initSolOmnibusAccount,
    newAccountWithLamports,
} from '../../../src';
import { testServerManager } from './testServerManager';
import { freePort } from './freePort';

const registerLogs = (page: Page) => {
    page.on('console', message => console.log(`Page log: ${message.text()}`));
    page.on('pageerror', error => {
        console.log(`Page error: ${error.message}`);
    });
    page.on('response', response => {
        console.log(
            `Received response: ${response.url()} - ${response.status()}`,
        );
    });
};

const DEBUG = true;

describe('RPC in browser', () => {
    const { merkleTree } = defaultTestStateTreeAccounts();

    let browser: Browser;
    let page: Page;

    beforeAll(async () => {
        try {
            await freePort();
            await testServerManager.startServer();

            const rpc = createRpc();
            const payer = await newAccountWithLamports(rpc, 1000005000, 100);
            const initAuthority = await newAccountWithLamports(rpc, 1e9);

            browser = await chromium.launch();
            page = await browser.newPage();

            if (DEBUG) registerLogs(page);

            await page.goto('http://localhost:6042/test-page.html');
            await page.waitForLoadState('networkidle');
            await initSolOmnibusAccount(rpc, initAuthority, initAuthority);
            await compressLamports(
                rpc,
                payer,
                1e9,
                payer.publicKey,
                merkleTree,
            );
        } catch (error) {
            console.log('error: ', error);
        }
    }, 15000);

    it.only('getCompressedAccountsByOwner', async () => {
        const result = await page.evaluate(async () => {
            //@ts-ignore
            console.log('@?? OK? window.stateless?', window.stateless);
            //@ts-ignore
            const sdk = window.stateless;
            const rpc: Rpc = sdk.createRpc();
            const payer = sdk.getTestKeypair(100);
            const compressedAccounts = await rpc.getCompressedAccountsByOwner(
                payer.publicKey,
            );
            if (!compressedAccounts)
                throw new Error('No compressed accounts found');
            return compressedAccounts;
        });
        assert.equal(result.length, 1);
    });

    it.only('getCompressedAccount', async () => {
        console.log('@getCompressedAccount');
        const result = await page.evaluate(async () => {
            console.log('@eval');
            //@ts-ignore
            console.log('@?? window.stateless?', window.stateless);
            //@ts-ignore
            const sdk = window.stateless;
            const rpc: Rpc = sdk.createRpc();
            const payer = sdk.getTestKeypair(100);
            console.log('payer: ', payer ? true : false);
            const compressedAccounts = await rpc.getCompressedAccountsByOwner(
                payer.publicKey,
            );
            console.log(
                'compressedAccounts: ',
                compressedAccounts ? true : false,
            );
            const hash = compressedAccounts[0].hash;
            console.log('@getCompressedAccount hash: ', hash);
            console.log('RPC??', rpc ? true : false, sdk ? true : false);
            //@ts-ignore
            const sdk2 = window.stateless;
            console.log('sdk2: ', sdk2);
            const rpc2: Rpc = sdk2.createRpc();
            console.log('rpc2: ', rpc2);
            let account: any;
            try {
                account = await rpc2.getCompressedAccount(bn(hash));
            } catch (error) {
                console.log('error: ', error);
                throw error;
            }
            console.log('account: ', account ? true : false);
            if (!account) throw new Error('No compressed account found');
            return { account, owner: payer.publicKey };
        });
        console.log('result: ', result);
        assert.isTrue(result.account.owner.equals(result.owner));
    });

    it('getMultipleCompressedAccounts', async () => {
        const result = await page.evaluate(async () => {
            //@ts-ignore
            const sdk = window.stateless;
            const rpc: Rpc = sdk.createRpc();
            const payer = sdk.getTestKeypair(100);
            const compressedAccounts = await rpc.getCompressedAccountsByOwner(
                payer.publicKey,
            );
            const hashes = compressedAccounts.map(account => bn(account.hash));
            const accounts = await rpc.getMultipleCompressedAccounts(hashes);
            if (!accounts || accounts.length === 0)
                throw new Error('No compressed accounts found');
            return accounts;
        });
        assert.isTrue(result.length > 0);
    });

    // it('getCompressedTokenAccountsByOwner', async () => {
    //     const result = await page.evaluate(async () => {
    //         //@ts-ignore
    //         const sdk = window.stateless;
    //         const rpc = sdk.createRpc();
    //         const payer = sdk.getTestKeypair(100);
    //         const compressedAccounts = await rpc.getCompressedAccountsByOwner(
    //             payer.publicKey,
    //         );
    //         const hash = compressedAccounts[0].hash;
    //         const accounts = await rpc.getCompressedTokenAccountsByOwner(owner);
    //         if (!accounts || accounts.length === 0)
    //             throw new Error('No token accounts found');
    //         return accounts;
    //     });
    //     assert.isTrue(result.length > 0);
    // });

    it('getHealth', async () => {
        const result = await page.evaluate(async () => {
            //@ts-ignore
            const sdk = window.stateless;
            const rpc: Rpc = sdk.createRpc();
            const health = await rpc.getHealth();
            if (!health) throw new Error('Health check failed');
            return health;
        });
        assert.equal(result, 'ok');
    });

    // afterAll(async () => {
    //     await browser.close();
    //     await testServerManager.stopServer();
    // });
});
