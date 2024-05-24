import { describe, it, assert, beforeAll, expect } from 'vitest';
import { Signer } from '@solana/web3.js';
import {
    STATE_MERKLE_TREE_NETWORK_FEE,
    ADDRESS_QUEUE_ROLLOVER_FEE,
    STATE_MERKLE_TREE_ROLLOVER_FEE,
    defaultTestStateTreeAccounts,
    ADDRESS_TREE_NETWORK_FEE,
} from '../../src/constants';
import { newAccountWithLamports } from '../../src/utils/test-utils';
import { Rpc } from '../../src/rpc';
import {
    LightSystemProgram,
    bn,
    compress,
    createAccount,
    createAccountWithLamports,
    decompress,
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
        const networkInFee = tx.in ? STATE_MERKLE_TREE_NETWORK_FEE : bn(0);

        /// Fee if the tx creates at least one address
        const networkAddressFee = tx.addr ? ADDRESS_TREE_NETWORK_FEE : bn(0);

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
    const { merkleTree } = defaultTestStateTreeAccounts();
    let rpc: Rpc;
    let payer: Signer;

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);
        payer = await newAccountWithLamports(rpc, 1e9, 256);
    });

    it('should create account with address', async () => {
        const preCreateAccountsBalance = await rpc.getBalance(payer.publicKey);

        await createAccount(
            rpc as TestRpc,
            payer,
            new Uint8Array([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
                19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
            ]),
            LightSystemProgram.programId,
        );

        await createAccountWithLamports(
            rpc as TestRpc,
            payer,
            new Uint8Array([
                1, 2, 255, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
                19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
            ]),
            0,
            LightSystemProgram.programId,
        );

        await createAccount(
            rpc as TestRpc,
            payer,
            new Uint8Array([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
                19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 1,
            ]),
            LightSystemProgram.programId,
        );

        await createAccount(
            rpc as TestRpc,
            payer,
            new Uint8Array([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
                19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 2,
            ]),
            LightSystemProgram.programId,
        );
        await expect(
            createAccount(
                rpc as TestRpc,
                payer,
                new Uint8Array([
                    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17,
                    18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 2,
                ]),
                LightSystemProgram.programId,
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

        await compress(rpc, payer, compressLamportsAmount, payer.publicKey);

        const compressedAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        assert.equal(compressedAccounts.length, 1);
        assert.equal(
            Number(compressedAccounts[0].lamports),
            compressLamportsAmount,
        );

        assert.equal(compressedAccounts[0].data, null);
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
            new Uint8Array([
                1, 255, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
                19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
            ]),
            100,
            LightSystemProgram.programId,
        );

        const postCreateAccountBalance = await rpc.getBalance(payer.publicKey);
        assert.equal(
            postCreateAccountBalance,
            postCompressBalance - txFees([{ in: 1, out: 2, addr: 1 }]),
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
            merkleTree,
        );

        const compressedAccounts = await rpc.getCompressedAccountsByOwner(
            payer.publicKey,
        );
        assert.equal(compressedAccounts.length, 1);
        assert.equal(
            Number(compressedAccounts[0].lamports),
            compressLamportsAmount,
        );

        assert.equal(compressedAccounts[0].data, null);
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
        assert.equal(compressedAccounts2.length, 1);
        assert.equal(
            Number(compressedAccounts2[0].lamports),
            compressLamportsAmount - decompressLamportsAmount,
        );
        await decompress(rpc, payer, 1, decompressRecipient, merkleTree);

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
