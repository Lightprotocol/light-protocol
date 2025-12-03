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
import BN from 'bn.js';
import { createUnwrapInstruction } from '../instructions/unwrap';
import {
    getSplInterfaceInfos,
    SplInterfaceInfo,
} from '../../utils/get-token-pool-infos';
import { getAssociatedTokenAddressInterface } from '../get-associated-token-address-interface';
import { loadAta as _loadAta } from './load-ata';

export interface UnwrapParams {
    rpc: Rpc;
    payer: Signer;
    owner: Signer;
    mint: PublicKey;
    destination: PublicKey;
    amount?: number | bigint | BN;
    splInterfaceInfo?: SplInterfaceInfo;
    confirmOptions?: ConfirmOptions;
}

export interface UnwrapResult {
    transactionSignature: TransactionSignature;
}

/**
 * Unwrap c-tokens to SPL tokens.
 *
 * This is the reverse of wrap: converts c-token balance to SPL/T22 balance.
 * Destination SPL/T22 ATA must already exist (same as SPL token transfer pattern).
 *
 * Flow:
 * 1. Consolidate all c-token balances (cold -> hot) via loadAta
 * 2. Transfer from c-token hot ATA to SPL ATA via token pool
 *
 * @param rpc                RPC connection
 * @param payer              Fee payer
 * @param owner              Owner of the c-token (signer)
 * @param mint               Mint address
 * @param destination        Destination SPL/T22 token account (must exist)
 * @param amount             Optional: specific amount to unwrap (defaults to all)
 * @param splInterfaceInfo   Optional: SPL interface info (will be fetched if not provided)
 * @param confirmOptions     Optional: confirm options
 *
 * @example
 * // Unwrap to existing SPL ATA
 * const splAta = getAssociatedTokenAddressSync(mint, owner.publicKey);
 * await unwrap(rpc, payer, owner, mint, splAta, 1000n);
 *
 * @returns Transaction signature
 */
export async function unwrap(
    rpc: Rpc,
    payer: Signer,
    owner: Signer,
    mint: PublicKey,
    destination: PublicKey,
    amount?: number | bigint | BN,
    splInterfaceInfo?: SplInterfaceInfo,
    confirmOptions?: ConfirmOptions,
): Promise<UnwrapResult> {
    // 1. Get SPL interface info if not provided
    let resolvedSplInterfaceInfo = splInterfaceInfo;
    if (!resolvedSplInterfaceInfo) {
        const splInterfaceInfos = await getSplInterfaceInfos(rpc, mint);
        resolvedSplInterfaceInfo = splInterfaceInfos.find(
            info => info.isInitialized,
        );

        if (!resolvedSplInterfaceInfo) {
            throw new Error(
                `No initialized SPL interface found for mint: ${mint.toBase58()}. ` +
                    `Please create an SPL interface via createSplInterface().`,
            );
        }
    }

    // 2. Verify destination exists (SPL token pattern - destination must exist)
    const destAtaInfo = await rpc.getAccountInfo(destination);
    if (!destAtaInfo) {
        throw new Error(
            `Destination account does not exist: ${destination.toBase58()}. ` +
                `Create it first using getOrCreateAssociatedTokenAccount or createAssociatedTokenAccountIdempotentInstruction.`,
        );
    }

    // 3. Load all tokens to c-token hot ATA first (consolidate cold -> hot)
    const ctokenAta = getAssociatedTokenAddressInterface(mint, owner.publicKey);
    await _loadAta(rpc, ctokenAta, owner, mint, payer, confirmOptions);

    // 4. Check c-token hot balance
    const ctokenAccountInfo = await rpc.getAccountInfo(ctokenAta);
    if (!ctokenAccountInfo) {
        throw new Error('No c-token ATA found after loading');
    }

    // Parse c-token account balance (offset 64 for amount in token account layout)
    const data = ctokenAccountInfo.data;
    const ctokenBalance = data.readBigUInt64LE(64);

    if (ctokenBalance === BigInt(0)) {
        throw new Error('No c-token balance to unwrap');
    }

    // 5. Determine amount to unwrap
    const unwrapAmount = amount ? BigInt(amount.toString()) : ctokenBalance;

    if (unwrapAmount > ctokenBalance) {
        throw new Error(
            `Insufficient c-token balance. Requested: ${unwrapAmount}, Available: ${ctokenBalance}`,
        );
    }

    // 6. Build unwrap instruction
    const ix = createUnwrapInstruction(
        ctokenAta,
        destination,
        owner.publicKey,
        mint,
        unwrapAmount,
        resolvedSplInterfaceInfo,
        payer.publicKey,
    );

    // 7. Build and send transaction
    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [owner]);

    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 300_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );

    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);

    return { transactionSignature: txId };
}
