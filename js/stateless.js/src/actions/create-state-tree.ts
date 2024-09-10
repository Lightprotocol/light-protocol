import { Keypair, PublicKey, TransactionSignature, ComputeBudgetProgram, Signer } from '@solana/web3.js';
import { LightRegistryProgram } from '../programs';
import { Rpc } from '../rpc';
import { buildAndSignTx, sendAndConfirmTx } from '../utils';

/**
 * Create a new state tree and nullifier queue
 *
 * @param rpc          Rpc to use
 * @param payer        Payer of transaction fees
 * @param index        Index for the state tree
 * @param programOwner Owner of the program (optional)
 * @param forester     Forester public key (optional)
 *
 * @return Signature of the confirmed transaction
 */
export async function createStateTreeAndNullifierQueue(
    rpc: Rpc,
    payer: Keypair,
    index: number
    // programOwner: PublicKey | null,
    // forester: PublicKey | null,
): Promise<TransactionSignature> {
    const merkleTreeKeypair = Keypair.generate();
    const queueKeypair = Keypair.generate();
    const cpiContextKeypair = Keypair.generate();
    const cpiAuthorityKeypair = Keypair.generate();

    console.log("merkleTreeKeypair address", merkleTreeKeypair.publicKey.toBase58());
    console.log("queueKeypair address", queueKeypair.publicKey.toBase58());
    console.log("cpiContextKeypair address", cpiContextKeypair.publicKey.toBase58());
    console.log("cpiAuthorityKeypair address", cpiAuthorityKeypair.publicKey.toBase58());

    console.log("merkleTreeKeypair secret key", merkleTreeKeypair.secretKey);
    console.log("queueKeypair secret key", queueKeypair.secretKey);
    console.log("cpiContextKeypair secret key", cpiContextKeypair.secretKey);
    console.log("cpiAuthorityKeypair secret key", cpiAuthorityKeypair.secretKey);


    const instructions = await LightRegistryProgram.createStateTreeAndNullifierQueueInstructions(
        rpc,
        payer,
        merkleTreeKeypair,
        queueKeypair,
        cpiContextKeypair,
        cpiAuthorityKeypair,
        null,
        null,
        index
    );

    const { blockhash } = await rpc.getLatestBlockhash();
    const signedTx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }), ...instructions],
        payer,
        blockhash,
        [merkleTreeKeypair, queueKeypair, cpiContextKeypair, cpiAuthorityKeypair]
    );

    const txId = await sendAndConfirmTx(rpc, signedTx);
    return txId;
}