import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import BN from 'bn.js';
import {
    sendAndConfirmTx,
    buildAndSignTx,
    Rpc,
    dedupeSigner,
    selectStateTreeInfo,
    toArray,
    TreeInfo,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../program';
import { getOrCreateAssociatedTokenAccount } from '@solana/spl-token';

import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../utils/get-token-pool-infos';

/**
 * Mint compressed tokens to a solana address from an external mint authority
 *
 * @param rpc                   Rpc to use
 * @param payer                 Fee payer
 * @param mint                  SPL Mint address
 * @param toPubkey              Address of the account to mint to
 * @param authority             Minting authority
 * @param amount                Amount to mint
 * @param outputStateTreeInfo   Optional: State tree account that the compressed
 *                              tokens should be inserted into. Defaults to a
 *                              shared state tree account.
 * @param tokenPoolInfo         Optional: Token pool info.
 * @param confirmOptions        Options for confirming the transaction
 *
 * @return Signature of the confirmed transaction
 */
export async function approveAndMintTo(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    toPubkey: PublicKey,
    authority: Signer,
    amount: number | BN,
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

    const authorityTokenAccount = await getOrCreateAssociatedTokenAccount(
        rpc,
        payer,
        mint,
        authority.publicKey,
        undefined,
        undefined,
        confirmOptions,
        tokenPoolInfo.tokenProgram,
    );

    const ixs = await CompressedTokenProgram.approveAndMintTo({
        feePayer: payer.publicKey,
        mint,
        authority: authority.publicKey,
        authorityTokenAccount: authorityTokenAccount.address,
        amount,
        toPubkey,
        outputStateTreeInfo,
        tokenPoolInfo,
    });

    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [authority]);

    const tx = buildAndSignTx(
        [
            ComputeBudgetProgram.setComputeUnitLimit({
                units: 150_000 + toArray(amount).length * 20_000,
            }),
            ...ixs,
        ],
        payer,
        blockhash,
        additionalSigners,
    );

    return await sendAndConfirmTx(rpc, tx, confirmOptions);
}
