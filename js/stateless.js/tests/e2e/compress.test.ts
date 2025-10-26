import { describe, it, assert, beforeAll, expect } from 'vitest';
import { Signer } from '@solana/web3.js';
import {
    STATE_MERKLE_TREE_NETWORK_FEE,
    ADDRESS_QUEUE_ROLLOVER_FEE,
    STATE_MERKLE_TREE_ROLLOVER_FEE,
    ADDRESS_TREE_NETWORK_FEE_V1,
    ADDRESS_TREE_NETWORK_FEE_V2,
    featureFlags,
} from '../../src/constants';
import { newAccountWithLamports } from '../../src/test-helpers/test-utils';
import { Rpc } from '../../src/rpc';
import {
    LightSystemProgram,
    TreeInfo,
    bn,
    compress,
    createAccount,
    createAccountWithLamports,
    decompress,
    selectStateTreeInfo,
} from '../../src';
import { TestRpc, getTestRpc } from '../../src/test-helpers/test-rpc';
import { WasmFactory } from '@lightprotocol/hasher.rs';

/// TODO: make available to developers via utils
function txFees(
    txs: {
        in: number;
        out: number;
        addr?: number;
        base?: number;
    }[],
): number {
    let totalFee = bn(0);

    txs.forEach(tx => {
        const solanaBaseFee = tx.base === 0 ? bn(0) : bn(tx.base || 5000);

        /// Fee per output
        const stateOutFee = STATE_MERKLE_TREE_ROLLOVER_FEE.mul(bn(tx.out));

        /// Fee per new address created
        const addrFee = tx.addr
            ? ADDRESS_QUEUE_ROLLOVER_FEE.mul(bn(tx.addr))
            : bn(0);

        /// Fee if the tx nullifies at least one input account
        const networkInFee = tx.in
            ? featureFlags.isV2()
                ? STATE_MERKLE_TREE_NETWORK_FEE
                : STATE_MERKLE_TREE_NETWORK_FEE.mul(bn(tx.in))
            : tx.out && featureFlags.isV2()
              ? STATE_MERKLE_TREE_NETWORK_FEE
              : bn(0);

        /// Network fee charged per address created
        const networkAddressFee = tx.addr
            ? ADDRESS_TREE_NETWORK_FEE_V1.mul(bn(tx.addr))
            : bn(0);
        // TODO: adapt once we use v2 address trees in tests.
        // tx.addr
        //   ? featureFlags.isV2()
        //       ? ADDRESS_TREE_NETWORK_FEE_V2.mul(bn(tx.addr))
        //       : ADDRESS_TREE_NETWORK_FEE_V1.mul(bn(tx.addr))
        //   : bn(0);
        totalFee = totalFee.add(
            solanaBaseFee
                .add(stateOutFee)
                .add(addrFee)
                .add(networkInFee)
                .add(networkAddressFee),
        );
    });

    return totalFee.toNumber();
}

/// TODO: add test case for payer != address
describe('compress', () => {
    let rpc: Rpc;
    let payer: Signer;
    let stateTreeInfo: TreeInfo;

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);
        payer = await newAccountWithLamports(rpc, 1e9, 256);
        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());
    });

    it('should create account with address', async () => {
        const preCreateAccountsBalance = await rpc.getBalance(payer.publicKey);

        await createAccount(
            rpc as TestRpc,
            payer,
            [
                new Uint8Array([
                    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
                    18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
                ]),
            ],
            LightSystemProgram.programId,
            undefined,
            stateTreeInfo,
        );

        await expect(
            createAccountWithLamports(
                rpc as TestRpc,
                payer,
                [
                    new Uint8Array([
                        1, 2, 255, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
                        17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30,
                        31, 32,
                    ]),
                ],
                0,
                LightSystemProgram.programId,
            ),
        ).rejects.toThrowError(
            'Neither input accounts nor outputStateTreeInfo are available',
        );

        // 0 lamports => 0 input accounts selected, so outputStateTreeInfo is required
        await createAccountWithLamports(
            rpc as TestRpc,
            payer,
            [
                new Uint8Array([
                    1, 2, 255, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
                    18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
                ]),
            ],
            0,
            LightSystemProgram.programId,
            undefined,
            stateTreeInfo,
        );

        await createAccount(
            rpc as TestRpc,
            payer,
            [
                new Uint8Array([
                    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
                    18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 1,
                ]),
            ],
            LightSystemProgram.programId,
            undefined,
            stateTreeInfo,
        );

        await createAccount(
            rpc as TestRpc,
            payer,
            [
                new Uint8Array([
                    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
                    18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 2,
                ]),
            ],
            LightSystemProgram.programId,
            undefined,
            stateTreeInfo,
        );
        await expect(
            createAccount(
                rpc as TestRpc,
                payer,
                [
                    new Uint8Array([
                        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
                        17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30,
                        31, 2,
                    ]),
                ],
                LightSystemProgram.programId,
                undefined,
                stateTreeInfo,
            ),
        ).rejects.toThrow();
        const postCreateAccountsBalance = await rpc.getBalance(payer.publicKey);
        assert.equal(
            postCreateAccountsBalance,
            preCreateAccountsBalance -
                txFees([
                    { in: 0, out: 1, addr: 1 },
                    { in: 0, out: 1, addr: 1 },
                    { in: 0, out: 1, addr: 1 },
                    { in: 0, out: 1, addr: 1 },
                ]),
        );
    });

    it('should compress lamports and create an account with address and lamports', async () => {
        payer = await newAccountWithLamports(rpc, 1e9, 256);

        const compressLamportsAmount = 1e7;
        const preCompressBalance = await rpc.getBalance(payer.publicKey);
        assert.equal(preCompressBalance, 1e9);

        await compress(
            rpc,
            payer,
            compressLamportsAmount,
            payer.publicKey,
            stateTreeInfo,
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
                txFees([{ in: 0, out: 1 }]),
        );

        await createAccountWithLamports(
            rpc as TestRpc,
            payer,
            [
                new Uint8Array([
                    1, 255, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
                    18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
                ]),
            ],
            100,
            LightSystemProgram.programId,
            undefined,
        );

        const postCreateAccountBalance = await rpc.getBalance(payer.publicKey);
        let expectedTxFees = txFees([{ in: 1, out: 2, addr: 1 }]);
        assert.equal(
            postCreateAccountBalance,
            postCompressBalance - expectedTxFees,
        );
    });

    it('should compress lamports and create an account with address and lamports', async () => {
        payer = await newAccountWithLamports(rpc, 1e9, 256);

        const compressLamportsAmount = 1e7;
        const preCompressBalance = await rpc.getBalance(payer.publicKey);
        assert.equal(preCompressBalance, 1e9);

        await compress(rpc, payer, compressLamportsAmount, payer.publicKey);

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
                txFees([{ in: 0, out: 1 }]),
        );

        /// Decompress
        const decompressLamportsAmount = 1e6;
        const decompressRecipient = payer.publicKey;

        await decompress(
            rpc,
            payer,
            decompressLamportsAmount,
            decompressRecipient,
        );

        const compressedAccounts2 = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        assert.equal(compressedAccounts2.items.length, 1);
        assert.equal(
            Number(compressedAccounts2.items[0].lamports),
            compressLamportsAmount - decompressLamportsAmount,
        );
        await decompress(rpc, payer, 1, decompressRecipient);

        const postDecompressBalance = await rpc.getBalance(decompressRecipient);
        assert.equal(
            postDecompressBalance,
            postCompressBalance +
                decompressLamportsAmount +
                1 -
                txFees([
                    { in: 1, out: 1 },
                    { in: 1, out: 1 },
                ]),
        );
    });
});
