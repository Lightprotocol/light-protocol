import { describe, it, assert, beforeAll } from 'vitest';
import {
    CompressedAccount,
    CompressedAccountWithMerkleContext,
    MerkleContext,
    bn,
} from '../../src/state';
import { sendAndConfirmTx, buildAndSignTx } from '../../src/utils';

import { Keypair, Signer } from '@solana/web3.js';
import { defaultTestStateTreeAccounts } from '../../src/constants';
import { getTestRpc, newAccountWithLamports } from '../../src/test-utils';
import { LightSystemProgram, Rpc } from '../../src';

/// TODO: add test case for payer != address
describe('compress', () => {
    const { merkleTree, nullifierQueue } = defaultTestStateTreeAccounts();
    let rpc: Rpc;
    let payer: Signer;
    let initAuthority: Signer;

    beforeAll(async () => {
        rpc = await getTestRpc();
        payer = await newAccountWithLamports(rpc, 1e9, 200);
        initAuthority = await newAccountWithLamports(rpc, 1e9);
    });

    it('should compress lamports and then decompress', async () => {
        const compressLamports = 20;
        const preCompressBalance = await rpc.getBalance(payer.publicKey);
        assert.equal(preCompressBalance, 1e9);

        const ix = await LightSystemProgram.initCompressedSolPda(
            initAuthority.publicKey,
        );
        const { blockhash: initBlockhash } = await rpc.getLatestBlockhash();
        const signedInitTx = buildAndSignTx([ix], initAuthority, initBlockhash);
        await sendAndConfirmTx(rpc, signedInitTx);

        const ixs = await LightSystemProgram.compress({
            payer: payer.publicKey,
            address: payer.publicKey,
            lamports: compressLamports,
            outputStateTree: merkleTree,
        });

        /// Send
        const { blockhash } = await rpc.getLatestBlockhash();
        const signedTx = buildAndSignTx(ixs, payer, blockhash);
        await sendAndConfirmTx(rpc, signedTx);

        rpc = await getTestRpc();

        // @ts-ignore
        const indexedEvents = await rpc.getParsedEvents();
        assert.equal(indexedEvents.length, 2);
        assert.equal(indexedEvents[0].inputCompressedAccounts.length, 0);
        assert.equal(indexedEvents[0].outputCompressedAccounts.length, 1);
        assert.equal(
            Number(indexedEvents[0].outputCompressedAccounts[0].lamports),
            compressLamports,
        );
        assert.equal(
            indexedEvents[0].outputCompressedAccounts[0].owner.toBase58(),
            payer.publicKey.toBase58(),
        );
        assert.equal(indexedEvents[0].outputCompressedAccounts[0].data, null);
        const postCompressBalance = await rpc.getBalance(payer.publicKey);
        assert.equal(
            postCompressBalance,
            preCompressBalance - compressLamports - 5000,
        );

        /// TODO: use test-rpc call to get the account
        const inputAccount: CompressedAccount =
            indexedEvents[0].outputCompressedAccounts[0];
        const inputAccountHash: number[] =
            indexedEvents[0].outputCompressedAccountHashes[0];
        const inputAccountLeafIndex: number =
            indexedEvents[0].outputLeafIndices[0];

        const proof = await rpc.getValidityProof([bn(inputAccountHash)]);

        const merkleCtx: MerkleContext = {
            merkleTree: merkleTree, // TODO: dynamic
            nullifierQueue: nullifierQueue, // TODO: dynamic
            hash: inputAccountHash,
            leafIndex: inputAccountLeafIndex,
        };
        const withCtx: CompressedAccountWithMerkleContext = {
            ...inputAccount,
            ...merkleCtx,
        };

        /// Decompress
        const decompressLamports = 15;
        const decompressRecipient = payer.publicKey;

        const decompressIx = await LightSystemProgram.decompress({
            payer: payer.publicKey,
            toAddress: decompressRecipient,
            outputStateTree: merkleTree,
            inputCompressedAccounts: [withCtx],
            recentValidityProof: proof.compressedProof,
            recentInputStateRootIndices: proof.rootIndices,
            lamports: decompressLamports,
        });

        const { blockhash: decompressBlockhash } =
            await rpc.getLatestBlockhash();
        const signedDecompressTx = buildAndSignTx(
            decompressIx,
            payer,
            decompressBlockhash,
        );
        await sendAndConfirmTx(rpc, signedDecompressTx);

        //@ts-ignore
        const indexedEvents2 = await rpc.getParsedEvents();
        assert.equal(indexedEvents2.length, 3);
        assert.equal(indexedEvents2[0].inputCompressedAccounts.length, 1);
        assert.equal(indexedEvents2[0].outputCompressedAccounts.length, 1);
        assert.equal(
            Number(indexedEvents2[0].outputCompressedAccounts[0].lamports),
            compressLamports - decompressLamports,
        );
        const postDecompressBalance = await rpc.getBalance(decompressRecipient);
        assert.equal(
            postDecompressBalance,
            postCompressBalance + decompressLamports - 5000,
        );
    });
});
