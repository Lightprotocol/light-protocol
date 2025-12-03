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
    getSplInterfaceInfos,
    selectSplInterfaceInfo,
    SplInterfaceInfo,
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
 * @param splInterfaceInfo      Optional: SPL interface info.
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
    splInterfaceInfo?: SplInterfaceInfo,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    outputStateTreeInfo =
        outputStateTreeInfo ??
        selectStateTreeInfo(await rpc.getStateTreeInfos());
    splInterfaceInfo =
        splInterfaceInfo ??
        selectSplInterfaceInfo(await getSplInterfaceInfos(rpc, mint));

    const authorityTokenAccount = await getOrCreateAssociatedTokenAccount(
        rpc,
        payer,
        mint,
        authority.publicKey,
        undefined,
        undefined,
        confirmOptions,
        splInterfaceInfo.tokenProgram,
    );

    const ixs = await CompressedTokenProgram.approveAndMintTo({
        feePayer: payer.publicKey,
        mint,
        authority: authority.publicKey,
        authorityTokenAccount: authorityTokenAccount.address,
        amount,
        toPubkey,
        outputStateTreeInfo,
        tokenPoolInfo: splInterfaceInfo,
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
