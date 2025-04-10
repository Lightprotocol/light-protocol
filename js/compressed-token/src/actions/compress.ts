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
    pickRandomTreeAndQueue,
    StateTreeInfo,
    selectStateTreeInfo,
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
 * @param payer                 Payer of the transaction fees
 * @param mint                  Mint of the compressed token
 * @param amount                Number of tokens to transfer
 * @param owner                 Owner of the compressed tokens.
 * @param sourceTokenAccount    Source (associated) token account
 * @param toAddress             Destination address of the recipient
 * @param outputStateTreeInfo   State tree account that the compressed tokens
 *                              should be inserted into. Defaults to a default
 *                              state tree account.
 * @param tokenPoolInfo         Token pool info
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
    outputStateTreeInfo?: StateTreeInfo,
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
                units: 600_000,
            }),
            compressIx,
        ],
        payer,
        blockhashCtx.blockhash,
        additionalSigners,
    );

    return await sendAndConfirmTx(rpc, signedTx, confirmOptions, blockhashCtx);
}
