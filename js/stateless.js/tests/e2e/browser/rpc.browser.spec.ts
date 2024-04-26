import { test, expect } from '@playwright/test';

import {
    Rpc,
    bn,
    compress,
    createRpc,
    defaultTestStateTreeAccounts,
    newAccountWithLamports,
} from '../../../src';

test.describe('RPC in browser', () => {
    const { merkleTree } = defaultTestStateTreeAccounts();

    test.beforeAll(async ({ page }) => {
        try {
            const rpc = createRpc();
            const payer = await newAccountWithLamports(rpc, 1000005000, 100);

            await page.goto(
                'http://localhost:4004/tests/e2e/browser/test-page.html',
            );
            await page.waitForFunction(
                () => (window as any).stateless !== undefined,
            );
            await compress(rpc, payer, 1e9, payer.publicKey, merkleTree);
        } catch (error) {
            console.log('error: ', error);
        }
    });

    test.only('getCompressedAccountsByOwner', async ({ page }) => {
        const result = await page.evaluate(async () => {
            // @ts-ignore
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
        expect(result.length).toEqual(1);
    });

    test('getCompressedAccount', async ({ page }) => {
        const result = await page.evaluate(async () => {
            //@ts-ignore
            const sdk = window.stateless;
            const rpc: Rpc = sdk.createRpc();
            const payer = sdk.getTestKeypair(100);

            const compressedAccounts = await rpc.getCompressedAccountsByOwner(
                payer.publicKey,
            );

            const hash = compressedAccounts[0].hash;
            //@ts-ignore
            const sdk2 = window.stateless;
            const rpc2: Rpc = sdk2.createRpc();
            let account: any;
            try {
                account = await rpc2.getCompressedAccount(bn(hash));
            } catch (error) {
                console.log('error: ', error);
                throw error;
            }
            if (!account) throw new Error('No compressed account found');
            return { account, owner: payer.publicKey };
        });
        expect(result.account.owner.equals(result.owner)).toBeTruthy();
    });

    test('getMultipleCompressedAccounts', async ({ page }) => {
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
        expect(result.length).toBeGreaterThan(0);
    });

    // TODO: enable
    // test('getCompressedTokenAccountsByOwner', async ({ page }) => {
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

    test('getHealth', async ({ page }) => {
        const result = await page.evaluate(async () => {
            //@ts-ignore
            const sdk = window.stateless;
            const rpc: Rpc = sdk.createRpc();
            const health = await rpc.getHealth();
            if (!health) throw new Error('Health check failed');
            return health;
        });
        expect(result).toEqual('ok');
    });
});
