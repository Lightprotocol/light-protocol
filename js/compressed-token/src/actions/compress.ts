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
    StateTreeInfo,
    pickStateTreeInfo,
    TreeType,
} from '@lightprotocol/stateless.js';

import BN from 'bn.js';

import { CompressedTokenProgram } from '../program';

/**
 * Compress SPL tokens
 *
 * @param rpc                       Rpc connection to use
 * @param payer                     Payer of the transaction fees
 * @param mint                      Mint of the compressed token
 * @param amount                    Number of tokens to transfer
 * @param owner                     Owner of the compressed tokens.
 * @param sourceTokenAccount        Source (associated) token account
 * @param toAddress                 Destination address of the recipient
 * @param outputStateTreeInfo    State tree context that the compressed
 *                                  tokens should be part of. Defaults to the
 *                                  default state tree context.
 * @param confirmOptions            Options for confirming the transaction
 *
 *
 * @return Signature of the confirmed transaction
 */
export async function compress(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    amount: number | BN | number[] | BN[],
    owner: Signer,
    sourceTokenAccount: PublicKey,
    toAddress: PublicKey | Array<PublicKey>,
    outputStateTreeInfo?: StateTreeInfo,
    confirmOptions?: ConfirmOptions,
    tokenProgramId?: PublicKey,
): Promise<TransactionSignature> {
    tokenProgramId = tokenProgramId
        ? tokenProgramId
        : await CompressedTokenProgram.get_mint_program_id(mint, rpc);

    if (!outputStateTreeInfo) {
        const stateTreeInfo = await rpc.getCachedActiveStateTreeInfos();
        outputStateTreeInfo = pickStateTreeInfo(
            stateTreeInfo,
            TreeType.StateV2,
        );
    }

    const compressIx = await CompressedTokenProgram.compress({
        payer: payer.publicKey,
        owner: owner.publicKey,
        source: sourceTokenAccount,
        toAddress,
        amount,
        mint,
        outputStateTreeInfo,
        tokenProgramId,
    });

    const blockhashCtx = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [owner]);
    const signedTx = buildAndSignTx(
        [
            ComputeBudgetProgram.setComputeUnitLimit({
                units: 550_000,
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
