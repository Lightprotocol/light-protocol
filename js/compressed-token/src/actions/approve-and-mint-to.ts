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
    pickStateTreeInfo,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../program';
import { getOrCreateAssociatedTokenAccount } from '@solana/spl-token';
import { StorageOptions } from '../types';

/**
 * Mint compressed tokens to a solana address from an external mint authority
 *
 * @param rpc            Rpc to use
 * @param payer          Payer of the transaction fees
 * @param mint           Mint for the account
 * @param destination    Address of the account to mint to
 * @param authority      Minting authority
 * @param amount         Amount to mint
 * @param storageOptions Options for storing the tokens accounts
 * @param confirmOptions Options for confirming the transaction
 *
 * @return Signature of the confirmed transaction
 */
export async function approveAndMintTo(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    destination: PublicKey,
    authority: Signer,
    amount: number | BN,
    storageOptions?: StorageOptions,
    confirmOptions?: ConfirmOptions,
    tokenProgramId?: PublicKey,
): Promise<TransactionSignature> {
    tokenProgramId = tokenProgramId
        ? tokenProgramId
        : await CompressedTokenProgram.get_mint_program_id(mint, rpc);

    const authorityTokenAccount = await getOrCreateAssociatedTokenAccount(
        rpc,
        payer,
        mint,
        authority.publicKey,
        undefined,
        undefined,
        confirmOptions,
        tokenProgramId,
    );

    if (!storageOptions) storageOptions = {};
    if (!storageOptions.stateTreeInfo) {
        const stateTreeInfos = await rpc.getCachedActiveStateTreeInfos();
        const info = pickStateTreeInfo(stateTreeInfos);
        storageOptions.stateTreeInfo = info;
    }
    if (!storageOptions.tokenPoolInfos) {
        const tokenPoolInfos = await rpc.getTokenPoolInfos(mint);
        storageOptions.tokenPoolInfos = tokenPoolInfos;
    }

    const ixs = await CompressedTokenProgram.approveAndMintTo({
        feePayer: payer.publicKey,
        mint,
        authority: authority.publicKey,
        authorityTokenAccount: authorityTokenAccount.address,
        amount,
        toPubkey: destination,
        outputStateTreeInfo: storageOptions.stateTreeInfo,
        tokenPoolInfos: storageOptions.tokenPoolInfos,
        tokenProgramId,
    });

    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [authority]);

    const tx = buildAndSignTx(
        [
            ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }),
            ...ixs,
        ],
        payer,
        blockhash,
        additionalSigners,
    );

    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);

    return txId;
}
