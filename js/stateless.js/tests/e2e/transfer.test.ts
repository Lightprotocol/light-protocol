import { describe, it, assert, beforeAll } from 'vitest';
import { CompressedProof_IdlType, Utxo, Utxo_IdlType } from '../../src/state';
import { sendAndConfirmTx, buildAndSignTx } from '../../src/utils';

import { Keypair, Signer } from '@solana/web3.js';
import { BN } from '@coral-xyz/anchor';
import { createExecuteCompressedInstruction } from '../../src/instruction/pack-nop-instruction';
import { defaultTestStateTreeAccounts } from '../../src/constants';
import { getTestRpc, newAccountWithLamports } from '../../src/test-utils';
import { Rpc } from '../../src';

describe('transfer', () => {
    const { merkleTree } = defaultTestStateTreeAccounts();
    let rpc: Rpc;
    let payer: Signer;
    let bob: Signer;

    beforeAll(async () => {
        rpc = await getTestRpc();
        payer = await newAccountWithLamports(rpc);
        bob = Keypair.generate();
    });
    // Note:
    // We don't compress SOL yet, therefore cannot spend utxos with value yet.
    // TODO: add one run with with inputUtxo where lamports: 0
    it('should send compressed lamports alice -> bob', async () => {
        const in_utxos: Utxo_IdlType[] = [];
        const out_utxos: Utxo[] = [
            {
                owner: bob.publicKey,
                lamports: new BN(0),
                data: null,
                address: null,
            },
            {
                owner: payer.publicKey,
                lamports: new BN(0),
                data: null,
                address: null,
            },
        ];

        const proof_mock: CompressedProof_IdlType = {
            a: Array.from({ length: 32 }, () => 0),
            b: Array.from({ length: 64 }, () => 0),
            c: Array.from({ length: 32 }, () => 0),
        };

        const ix = await createExecuteCompressedInstruction(
            payer.publicKey,
            in_utxos,
            out_utxos,
            [],
            [],
            [merkleTree, merkleTree],
            [],
            proof_mock,
        );
        const ixs = [ix];

        /// Send
        const { blockhash } = await rpc.getLatestBlockhash();
        const signedTx = buildAndSignTx(ixs, payer, blockhash);
        await sendAndConfirmTx(rpc, signedTx);

        /// @ts-ignore
        const indexedEvents = await rpc.getParsedEvents();

        assert.equal(indexedEvents.length, 1);
        assert.equal(indexedEvents[0].inUtxos.length, 0);
        assert.equal(indexedEvents[0].outUtxos.length, 2);
        assert.equal(Number(indexedEvents[0].outUtxos[0].lamports), 0);
        assert.equal(Number(indexedEvents[0].outUtxos[1].lamports), 0);
        assert.equal(
            indexedEvents[0].outUtxos[0].owner.toBase58(),
            bob.publicKey.toBase58(),
        );
        assert.equal(
            indexedEvents[0].outUtxos[1].owner.toBase58(),
            payer.publicKey.toBase58(),
        );
        assert.equal(indexedEvents[0].outUtxos[0].data, null);
        assert.equal(indexedEvents[0].outUtxos[1].data, null);
    });
});
