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
    toArray,
    TreeInfo,
} from '@lightprotocol/stateless.js';
import BN from 'bn.js';
import { CompressedTokenProgram } from '../program';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../utils/get-token-pool-infos';

/**
 * Compress SPL tokens
 *
 * @param rpc                   Rpc connection to use
 * @param payer                 Fee payer
 * @param mint                  SPL Mint address
 * @param amount                Number of tokens to compress.
 * @param owner                 Owner of the SPL token account.
 * @param sourceTokenAccount    Source SPL token account. (ATA)
 * @param toAddress             Recipient owner address.
 * @param outputStateTreeInfo   Optional: State tree account that the compressed
 *                              tokens should be inserted into. Defaults to a
 *                              shared state tree account.
 * @param tokenPoolInfo         Optional: Token pool info.
 * @param confirmOptions        Options for confirming the transaction
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
    outputStateTreeInfo?: TreeInfo,
    tokenPoolInfo?: TokenPoolInfo,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    outputStateTreeInfo =
        outputStateTreeInfo ??
        selectStateTreeInfo(await rpc.getStateTreeInfos());
    tokenPoolInfo =
        tokenPoolInfo ??
        selectTokenPoolInfo(await getTokenPoolInfos(rpc, mint));

    const compressIx = await CompressedTokenProgram.compress({
        payer: payer.publicKey,
        owner: owner.publicKey,
        source: sourceTokenAccount,
        toAddress,
        amount,
        mint,
        outputStateTreeInfo,
        tokenPoolInfo,
    });

    const blockhashCtx = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [owner]);
    const signedTx = buildAndSignTx(
        [
            ComputeBudgetProgram.setComputeUnitLimit({
                units: 130_000 + toArray(amount).length * 20_000,
            }),
            compressIx,
        ],
        payer,
        blockhashCtx.blockhash,
        additionalSigners,
    );

    return await sendAndConfirmTx(rpc, signedTx, confirmOptions, blockhashCtx);
}
