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
    pickRandomTreeAndQueue,
    StateTreeInfo,
    selectStateTreeInfo,
    toArray,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../program';
import { getOrCreateAssociatedTokenAccount } from '@solana/spl-token';
import { isSingleTokenPoolInfo, StorageOptions } from '../types';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    selectTokenPoolInfosForDecompression,
    TokenPoolInfo,
} from '../utils/get-token-pool-infos';

async function getStorageOptions(
    rpc: Rpc,
    mint: PublicKey,
    decompressAmount?: number | BN,
): Promise<StorageOptions> {
    const res = await Promise.all([
        rpc.getCachedActiveStateTreeInfos(),
        getTokenPoolInfos(rpc, mint),
    ]);

    return {
        stateTreeInfo: selectStateTreeInfo(res[0]),
        tokenPoolInfos: decompressAmount
            ? selectTokenPoolInfosForDecompression(res[1], decompressAmount)
            : selectTokenPoolInfo(res[1]),
    };
}

/**
 * Mint compressed tokens to a solana address from an external mint authority
 *
 * @param rpc                   Rpc to use
 * @param payer                 Payer of the transaction fees
 * @param mint                  Mint for the account
 * @param toPubkey              Address of the account to mint to
 * @param authority             Minting authority
 * @param amount                Amount to mint
 * @param outputStateTreeInfo   State tree info
 * @param tokenPoolInfo         Token pool info
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
    outputStateTreeInfo?: StateTreeInfo,
    tokenPoolInfo?: TokenPoolInfo,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    outputStateTreeInfo =
        outputStateTreeInfo ??
        selectStateTreeInfo(await rpc.getCachedActiveStateTreeInfos());
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
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 600_000 }), ...ixs],
        payer,
        blockhash,
        additionalSigners,
    );

    return await sendAndConfirmTx(rpc, tx, confirmOptions);
}
