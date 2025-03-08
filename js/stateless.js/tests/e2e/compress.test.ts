import { describe, it, assert, beforeAll, expect } from 'vitest';
import { Signer } from '@solana/web3.js';
import { newAccountWithLamports } from '../../src/test-helpers/test-utils';
import { Rpc } from '../../src/rpc';
import {
    LightSystemProgram,
    StateTreeInfo,
    TreeType,
    compress,
    createAccount,
    createAccountWithLamports,
    decompress,
} from '../../src';
import { TestRpc, getTestRpc } from '../../src/test-helpers/test-rpc';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import {
    getStateTreeContextByTypeForTest,
    txFees,
    txFeesV2Accounts,
} from './shared';
import { randomBytes } from '@noble/hashes/utils';

describe.each([TreeType.StateV1, TreeType.StateV2])(
    'Test with %s state tree',
    treeType => {
        let rpc: Rpc;
        let payer: Signer;
        let outputStateTreeInfo: StateTreeInfo;

        let feeFunction =
            treeType === TreeType.StateV1 ? txFees : txFeesV2Accounts;

        beforeAll(async () => {
            const lightWasm = await WasmFactory.getInstance();
            rpc = await getTestRpc(lightWasm);
            payer = await newAccountWithLamports(rpc, 1e9, 256);
            outputStateTreeInfo = await getStateTreeContextByTypeForTest(
                rpc,
                treeType,
            );
        });

        it('should create multiple accounts with addresses', async () => {
            const preCreateAccountsBalance = await rpc.getBalance(
                payer.publicKey,
            );

            await createAccount(
                rpc as TestRpc,
                payer,
                [new Uint8Array(randomBytes(32))],
                LightSystemProgram.programId,
                undefined,
                undefined,
                outputStateTreeInfo,
            );

            await createAccountWithLamports(
                rpc as TestRpc,
                payer,
                [new Uint8Array(randomBytes(32))],
                0,
                LightSystemProgram.programId,
                undefined,
                undefined,
                outputStateTreeInfo,
            );

            await createAccount(
                rpc as TestRpc,
                payer,
                [new Uint8Array(randomBytes(32))],
                LightSystemProgram.programId,
                undefined,
                undefined,
                outputStateTreeInfo,
            );

            let seed = new Uint8Array(randomBytes(32));
            await createAccount(
                rpc as TestRpc,
                payer,
                [seed],
                LightSystemProgram.programId,
                undefined,
                undefined,
                outputStateTreeInfo,
            );
            await expect(
                createAccount(
                    rpc as TestRpc,
                    payer,
                    [seed],
                    LightSystemProgram.programId,
                    undefined,
                    undefined,
                    outputStateTreeInfo,
                ),
            ).rejects.toThrow();
            const postCreateAccountsBalance = await rpc.getBalance(
                payer.publicKey,
            );
            assert.equal(
                postCreateAccountsBalance,
                preCreateAccountsBalance -
                    feeFunction([
                        { in: 0, out: 1, addr: 1 },
                        { in: 0, out: 1, addr: 1 },
                        { in: 0, out: 1, addr: 1 },
                        { in: 0, out: 1, addr: 1 },
                    ]),
            );
        });

        it('should compress and create an account with address (v1: and lamports)', async () => {
            payer = await newAccountWithLamports(rpc, 1e9, 256);

            const compressLamportsAmount = 1e7;
            const preCompressBalance = await rpc.getBalance(payer.publicKey);
            assert.equal(preCompressBalance, 1e9);

            await compress(
                rpc,
                payer,
                compressLamportsAmount,
                payer.publicKey,
                outputStateTreeInfo,
            );

            const compressedAccounts = await rpc.getCompressedAccountsByOwner(
                payer.publicKey,
            );
            assert.equal(compressedAccounts.items.length, 1);
            assert.equal(
                Number(compressedAccounts.items[0].lamports),
                compressLamportsAmount,
            );

            assert.equal(compressedAccounts.items[0].data, null);
            const postCompressBalance = await rpc.getBalance(payer.publicKey);
            assert.equal(
                postCompressBalance,
                preCompressBalance -
                    compressLamportsAmount -
                    feeFunction([{ in: 0, out: 1 }]),
            );

            if (treeType === TreeType.StateV1) {
                await createAccountWithLamports(
                    rpc as TestRpc,
                    payer,
                    [new Uint8Array(randomBytes(32))],
                    100,
                    LightSystemProgram.programId,
                    undefined,
                    undefined,
                    outputStateTreeInfo,
                );
            } else {
                await createAccount(
                    rpc as TestRpc,
                    payer,
                    [new Uint8Array(randomBytes(32))],
                    LightSystemProgram.programId,
                    undefined,
                    undefined,
                    outputStateTreeInfo,
                );
            }
            const postCreateAccountBalance = await rpc.getBalance(
                payer.publicKey,
            );
            assert.equal(
                postCreateAccountBalance,
                postCompressBalance -
                    feeFunction([
                        {
                            in: treeType === TreeType.StateV2 ? 0 : 1,
                            out: treeType === TreeType.StateV2 ? 1 : 2,
                            addr: 1,
                        },
                    ]),
            );
        });

        it('should compress lamports and decompress twice', async () => {
            payer = await newAccountWithLamports(rpc, 1e9, 256);

            const compressLamportsAmount = 1e7;
            const preCompressBalance = await rpc.getBalance(payer.publicKey);
            assert.equal(preCompressBalance, 1e9);

            await compress(
                rpc,
                payer,
                compressLamportsAmount,
                payer.publicKey,
                outputStateTreeInfo,
            );

            const compressedAccounts = await rpc.getCompressedAccountsByOwner(
                payer.publicKey,
            );
            assert.equal(compressedAccounts.items.length, 1);
            assert.equal(
                Number(compressedAccounts.items[0].lamports),
                compressLamportsAmount,
            );

            assert.equal(compressedAccounts.items[0].data, null);
            const postCompressBalance = await rpc.getBalance(payer.publicKey);
            assert.equal(
                postCompressBalance,
                preCompressBalance -
                    compressLamportsAmount -
                    feeFunction([{ in: 0, out: 1 }]),
            );

            /// Decompress
            const decompressLamportsAmount = 1e6;
            const decompressRecipient = payer.publicKey;

            await decompress(
                rpc,
                payer,
                decompressLamportsAmount,
                decompressRecipient,
                outputStateTreeInfo,
            );

            const compressedAccounts2 = await rpc.getCompressedAccountsByOwner(
                payer.publicKey,
            );
            assert.equal(compressedAccounts2.items.length, 1);
            assert.equal(
                Number(compressedAccounts2.items[0].lamports),
                compressLamportsAmount - decompressLamportsAmount,
            );
            await decompress(
                rpc,
                payer,
                1,
                decompressRecipient,
                outputStateTreeInfo,
            );

            const postDecompressBalance =
                await rpc.getBalance(decompressRecipient);
            const fixFee = treeType === TreeType.StateV1 ? 299 : 0; // TODO: investigate the need for this.

            assert.equal(
                postDecompressBalance,
                postCompressBalance +
                    decompressLamportsAmount +
                    fixFee +
                    1 -
                    feeFunction([
                        { in: 1, out: 1 },
                        { in: 1, out: 1 }, // 2
                    ]),
            );
        });
    },
);
