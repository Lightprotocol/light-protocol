import { PublicKey, Signer, TransactionSignature } from '@solana/web3.js';
import {
    sendAndConfirmTx,
    buildAndSignTx,
    Rpc,
    dedupeSigner,
} from '@lightprotocol/stateless.js';

import { CompressedTokenProgram } from '../program';

/**
 * Create a lookup table for the token program's default accounts
 *
 * @param rpc                   Rpc connection to use
 * @param payer                 Fee payer
 * @param authority             Authority of the lookup table
 * @param mints                 Optional array of mint public keys to include in
 *                              the lookup table
 * @param additionalAccounts    Optional array of additional account public keys
 *                              to include in the lookup table
 *
 * @return Object with transaction signatures and the address of the created
 *         lookup table
 */
export async function createTokenProgramLookupTable(
    rpc: Rpc,
    payer: Signer,
    authority: Signer,
    mints?: PublicKey[],
    additionalAccounts?: PublicKey[],
): Promise<{ txIds: TransactionSignature[]; address: PublicKey }> {
    const recentSlot = await rpc.getSlot('finalized');
    const { instructions, address } =
        await CompressedTokenProgram.createTokenProgramLookupTable({
            payer: payer.publicKey,
            authority: authority.publicKey,
            mints,
            remainingAccounts: additionalAccounts,
            recentSlot,
        });

    const additionalSigners = dedupeSigner(payer, [authority]);
    const txIds = [];

    for (const instruction of instructions) {
        const blockhashCtx = await rpc.getLatestBlockhash();
        const signedTx = buildAndSignTx(
            [instruction],
            payer,
            blockhashCtx.blockhash,
            additionalSigners,
        );
        const txId = await sendAndConfirmTx(
            rpc,
            signedTx,
            { commitment: 'finalized' },
            blockhashCtx,
        );
        txIds.push(txId);
    }

    return { txIds, address };
}
