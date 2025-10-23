import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import {
    Rpc,
    buildAndSignTx,
    sendAndConfirmTx,
    dedupeSigner,
} from '@lightprotocol/stateless.js';
import { createWrapInstruction } from '../instructions/wrap';
import {
    getTokenPoolInfos,
    TokenPoolInfo,
} from '../../utils/get-token-pool-infos';

// Keep old interface type for backwards compatibility export
export interface WrapParams {
    rpc: Rpc;
    payer: Signer;
    source: PublicKey;
    destination: PublicKey;
    owner: Signer;
    mint: PublicKey;
    amount: bigint;
    tokenPoolInfo?: TokenPoolInfo;
    confirmOptions?: ConfirmOptions;
}

export interface WrapResult {
    transactionSignature: TransactionSignature;
}

/**
 * Wrap tokens from an SPL/T22 account to a CToken account.
 *
 * This is an agnostic action that takes explicit account addresses (spl-token style).
 * Use getAssociatedTokenAddressSync() to derive ATA addresses if needed.
 *
 * @param rpc             RPC connection
 * @param payer           Fee payer
 * @param source          Source SPL/T22 token account (any token account, not just ATA)
 * @param destination     Destination CToken account (any CToken account, not just ATA)
 * @param owner           Owner/authority of the source account (must sign)
 * @param mint            Mint address
 * @param amount          Amount to wrap
 * @param tokenPoolInfo   Optional: Token pool info (will be fetched if not provided)
 * @param confirmOptions  Optional: Confirm options
 *
 * @example
 * const splAta = getAssociatedTokenAddressSync(mint, owner.publicKey, false, TOKEN_PROGRAM_ID);
 * const ctokenAta = getATAAddressInterface(mint, owner.publicKey); // defaults to CToken
 *
 * await wrap(
 *   rpc,
 *   payer,
 *   splAta,
 *   ctokenAta,
 *   owner,
 *   mint,
 *   1000n,
 * );
 *
 * @returns Transaction signature
 */
export async function wrap(
    rpc: Rpc,
    payer: Signer,
    source: PublicKey,
    destination: PublicKey,
    owner: Signer,
    mint: PublicKey,
    amount: bigint,
    tokenPoolInfo?: TokenPoolInfo,
    confirmOptions?: ConfirmOptions,
): Promise<WrapResult> {
    // Get token pool info if not provided
    let resolvedTokenPoolInfo = tokenPoolInfo;
    if (!resolvedTokenPoolInfo) {
        const tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
        resolvedTokenPoolInfo = tokenPoolInfos.find(info => info.isInitialized);

        if (!resolvedTokenPoolInfo) {
            throw new Error(
                `No initialized token pool found for mint: ${mint.toBase58()}. ` +
                    `Please create a token pool via createTokenPool().`,
            );
        }
    }

    // Build wrap instruction
    const ix = createWrapInstruction(
        source,
        destination,
        owner.publicKey,
        mint,
        amount,
        resolvedTokenPoolInfo,
        payer.publicKey,
    );

    // Build and send transaction
    const { blockhash } = await rpc.getLatestBlockhash();

    const additionalSigners = dedupeSigner(payer, [owner]);

    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 200_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );

    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);

    return { transactionSignature: txId };
}
