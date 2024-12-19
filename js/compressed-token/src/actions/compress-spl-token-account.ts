import {
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
    ComputeBudgetProgram,
} from '@solana/web3.js';
import {
    sendAndConfirmTx,
    buildAndSignTx,
    Rpc,
    dedupeSigner,
} from '@lightprotocol/stateless.js';

import BN from 'bn.js';

import { CompressedTokenProgram } from '../program';

/**
 * Compress SPL tokens into compressed token format
 *
 * @param rpc                   Rpc connection to use
 * @param payer                 Payer of the transaction fees
 * @param mint                  Mint of the token to compress
 * @param owner                 Owner of the token account
 * @param tokenAccount         Token account to compress
 * @param outputStateTree       State tree to insert the compressed token account into
 * @param remainingAmount      Optional: amount to leave in token account. Default: 0
 * @param confirmOptions       Options for confirming the transaction
 *
 * @return Signature of the confirmed transaction
 */
export async function compressSplTokenAccount(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    owner: Signer,
    tokenAccount: PublicKey,
    outputStateTree: PublicKey,
    remainingAmount?: BN,
    confirmOptions?: ConfirmOptions,
    tokenProgramId?: PublicKey,
): Promise<TransactionSignature> {
    tokenProgramId = tokenProgramId
        ? tokenProgramId
        : await CompressedTokenProgram.get_mint_program_id(mint, rpc);

    const compressIx = await CompressedTokenProgram.compressSplTokenAccount({
        feePayer: payer.publicKey,
        authority: owner.publicKey,
        tokenAccount,
        mint,
        remainingAmount,
        outputStateTree,
        tokenProgramId,
    });

    const blockhashCtx = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [owner]);
    const signedTx = buildAndSignTx(
        [
            ComputeBudgetProgram.setComputeUnitLimit({
                units: 1_000_000,
            }),
            compressIx,
        ],
        payer,
        blockhashCtx.blockhash,
        additionalSigners,
    );
    const txId = await sendAndConfirmTx(
        rpc,
        signedTx,
        confirmOptions,
        blockhashCtx,
    );
    return txId;
}
