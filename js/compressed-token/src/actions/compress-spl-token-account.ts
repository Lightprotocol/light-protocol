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
    selectStateTreeInfo,
    TreeInfo,
} from '@lightprotocol/stateless.js';
import BN from 'bn.js';
import {
    getSplInterfaceInfos,
    selectSplInterfaceInfo,
    SplInterfaceInfo,
} from '../utils/get-token-pool-infos';
import { CompressedTokenProgram } from '../program';

/**
 * Compress SPL tokens into compressed token format
 *
 * @param rpc                   Rpc connection to use
 * @param payer                 Fee payer
 * @param mint                  SPL Mint address
 * @param owner                 Owner of the token account
 * @param tokenAccount          Token account to compress
 * @param remainingAmount       Optional: amount to leave in token account.
 *                              Default: 0
 * @param outputStateTreeInfo   Optional: State tree account that the compressed
 *                              account into
 * @param splInterfaceInfo      Optional: SPL interface info.
 * @param confirmOptions        Options for confirming the transaction

 *
 * @return Signature of the confirmed transaction
 */
export async function compressSplTokenAccount(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    owner: Signer,
    tokenAccount: PublicKey,
    remainingAmount?: BN,
    outputStateTreeInfo?: TreeInfo,
    splInterfaceInfo?: SplInterfaceInfo,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    outputStateTreeInfo =
        outputStateTreeInfo ??
        selectStateTreeInfo(await rpc.getStateTreeInfos());
    splInterfaceInfo =
        splInterfaceInfo ??
        selectSplInterfaceInfo(await getSplInterfaceInfos(rpc, mint));

    const compressIx = await CompressedTokenProgram.compressSplTokenAccount({
        feePayer: payer.publicKey,
        authority: owner.publicKey,
        tokenAccount,
        mint,
        remainingAmount,
        outputStateTreeInfo,
        tokenPoolInfo: splInterfaceInfo,
    });

    const blockhashCtx = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [owner]);

    const signedTx = buildAndSignTx(
        [
            ComputeBudgetProgram.setComputeUnitLimit({
                units: 150_000,
            }),
            compressIx,
        ],
        payer,
        blockhashCtx.blockhash,
        additionalSigners,
    );

    return await sendAndConfirmTx(rpc, signedTx, confirmOptions, blockhashCtx);
}
