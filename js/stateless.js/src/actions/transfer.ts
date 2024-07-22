import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';

import { BN } from '@coral-xyz/anchor';
import {
    LightSystemProgram,
    selectMinCompressedSolAccountsForTransfer,
} from '../programs';
import { convertMerkleProofsWithContextToHex, proverRequest, Rpc } from '../rpc';

import { bn } from '../state';
import { buildAndSignTx, sendAndConfirmTx } from '../utils';
import { access } from 'fs';

/**
 * Transfer compressed lamports from one owner to another
 *
 * @param rpc            Rpc to use
 * @param payer          Payer of transaction fees
 * @param lamports       Number of lamports to transfer
 * @param owner          Owner of the compressed lamports
 * @param toAddress      Destination address of the recipient
 * @param merkleTree     State tree account that the compressed lamports should be
 *                       inserted into. Defaults to the default state tree account.
 * @param confirmOptions Options for confirming the transaction
 *
 *
 * @return Signature of the confirmed transaction
 */
export async function transfer(
    rpc: Rpc,
    payer: Signer,
    lamports: number | BN,
    owner: Signer,
    toAddress: PublicKey,
    /// TODO: allow multiple
    merkleTree?: PublicKey,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    lamports = bn(lamports);
    const compressedAccounts = await rpc.getCompressedAccountsByOwner(
        owner.publicKey,
    );

    const [inputAccounts] = selectMinCompressedSolAccountsForTransfer(
        compressedAccounts,
        lamports,
    );

    
    // rpc.getValidityProof_direct(inputAccounts.map(account => bn(account.hash))).then((proof) => {
    //     console.log("Proof: ", proof)
    // });

    let accountHashes = inputAccounts.map(account => bn(account.hash).toArray("be", 32));
    console.log("Account Hashes: ", accountHashes);

    let merkleProofsWithContext = await rpc.getMultipleCompressedAccountProofs(inputAccounts.map(account => bn(account.hash)));
    // console.log("Proofs: ", merkleProofsWithContext);

    merkleProofsWithContext.forEach((proof) => {
        console.log("Proof: ", proof.merkleProof.map(proof => proof.toArray("be", 32)));
    });

    const inputs = convertMerkleProofsWithContextToHex(
        merkleProofsWithContext,
    );



    const compressedProof = await proverRequest(
        'http://localhost:3001',
        'inclusion',
        inputs,
        false,
    );

    const proof = {
        compressedProof,
        roots: merkleProofsWithContext.map(proof => proof.root),
        rootIndices: merkleProofsWithContext.map(
            proof => proof.rootIndex,
        ),
        leafIndices: merkleProofsWithContext.map(
            proof => proof.leafIndex,
        ),
        leaves: merkleProofsWithContext.map(proof => bn(proof.hash)),
        merkleTrees: merkleProofsWithContext.map(
            proof => proof.merkleTree,
        ),
        nullifierQueues: merkleProofsWithContext.map(
            proof => proof.nullifierQueue,
        ),
    };

    console.log("Proofs: ", proof);
    
    proof.roots.forEach((root) => {
        console.log("Root: ", root.toArray("be", 32));
    });

    proof.leaves.forEach((leaf) => {
        console.log("Leaf: ", leaf.toArray("be", 32));
    });

    // const proof = await rpc.getValidityProof(
    //     inputAccounts.map(account => bn(account.hash)),
    // );

    const ix = await LightSystemProgram.transfer({
        payer: payer.publicKey,
        inputCompressedAccounts: inputAccounts,
        toAddress,
        lamports,
        recentInputStateRootIndices: proof.rootIndices,
        recentValidityProof: proof.compressedProof,
        outputStateTrees: merkleTree,
    });

    const { blockhash } = await rpc.getLatestBlockhash();
    const signedTx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }), ix],
        payer,
        blockhash,
    );
    const txId = await sendAndConfirmTx(rpc, signedTx, confirmOptions);
    return txId;
}
