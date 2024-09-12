import { Keypair, PublicKey, TransactionSignature, ComputeBudgetProgram, Signer } from '@solana/web3.js';
import { LightRegistryProgram } from '../programs';
import { Rpc } from '../rpc';
import { buildAndSignTx, sendAndConfirmTx } from '../utils';

/**
 * Create a new state tree and nullifier queue
 *
 * @param rpc          Rpc to use
 * @param payer        Payer of transaction fees
 * @param index        Index for the state tree *
 * @return Signature of the confirmed transaction
 */
export async function createStateTree(
    rpc: Rpc,
    payer: Keypair,
    index: number
): Promise<TransactionSignature> {
    const merkleTreeKeypair = Keypair.generate();
    const queueKeypair = Keypair.generate();
    const cpiContextKeypair = Keypair.generate();

    console.log("merkleTreeKeypair address", merkleTreeKeypair.publicKey.toBase58());
    console.log("queueKeypair address", queueKeypair.publicKey.toBase58());
    console.log("cpiContextKeypair address", cpiContextKeypair.publicKey.toBase58());

    console.log("merkleTreeKeypair secret key", merkleTreeKeypair.secretKey);
    console.log("queueKeypair secret key", queueKeypair.secretKey);
    console.log("cpiContextKeypair secret key", cpiContextKeypair.secretKey);



    const instructions = await LightRegistryProgram.createStateTreeAndNullifierQueueInstructions(
        rpc,
        payer,
        merkleTreeKeypair,
        queueKeypair,
        cpiContextKeypair,
        null,
        null,
        index
    );

    const { blockhash } = await rpc.getLatestBlockhash();
    const signedTx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }), ...instructions],
        payer,
        blockhash,
        [merkleTreeKeypair, queueKeypair, cpiContextKeypair]
    );

    console.log("signedTx", signedTx);

    const txId = await sendAndConfirmTx(rpc, signedTx, { skipPreflight: false});
    console.log("txId", txId);
    return txId;
}