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
import { getMint } from '@solana/spl-token';
import BN from 'bn.js';
import { createUnwrapInstruction } from '../instructions/unwrap';
import {
    getSplInterfaceInfos,
    SplInterfaceInfo,
} from '../../utils/get-token-pool-infos';
import { getAssociatedTokenAddressInterface } from '../get-associated-token-address-interface';
import { loadAta as _loadAta } from './load-ata';

/**
 * Unwrap c-tokens to SPL tokens.
 *
 * @param rpc                RPC connection
 * @param payer              Fee payer
 * @param destination        Destination SPL/T22 token account
 * @param owner              Owner of the c-token (signer)
 * @param mint               Mint address
 * @param amount             Amount to unwrap (defaults to all)
 * @param splInterfaceInfo   SPL interface info
 * @param confirmOptions     Confirm options
 *
 * @returns Transaction signature
 */
export async function unwrap(
    rpc: Rpc,
    payer: Signer,
    destination: PublicKey,
    owner: Signer,
    mint: PublicKey,
    amount?: number | bigint | BN,
    splInterfaceInfo?: SplInterfaceInfo,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
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

    const destAtaInfo = await rpc.getAccountInfo(destination);
    if (!destAtaInfo) {
        throw new Error(
            `Destination account does not exist: ${destination.toBase58()}. ` +
                `Create it first using getOrCreateAssociatedTokenAccount or createAssociatedTokenAccountIdempotentInstruction.`,
        );
    }

    // Load all tokens to c-token hot ATA
    const ctokenAta = getAssociatedTokenAddressInterface(mint, owner.publicKey);
    await _loadAta(rpc, ctokenAta, owner, mint, payer, confirmOptions);

    // Check c-token hot balance
    const ctokenAccountInfo = await rpc.getAccountInfo(ctokenAta);
    if (!ctokenAccountInfo) {
        throw new Error('No c-token ATA found after loading');
    }

    // Parse c-token account balance
    const data = ctokenAccountInfo.data;
    const ctokenBalance = data.readBigUInt64LE(64);

    if (ctokenBalance === BigInt(0)) {
        throw new Error('No c-token balance to unwrap');
    }

    const unwrapAmount = amount ? BigInt(amount.toString()) : ctokenBalance;

    if (unwrapAmount > ctokenBalance) {
        throw new Error(
            `Insufficient c-token balance. Requested: ${unwrapAmount}, Available: ${ctokenBalance}`,
        );
    }

    // Get mint info to get decimals
    const mintInfo = await getMint(
        rpc,
        mint,
        undefined,
        resolvedSplInterfaceInfo.tokenProgram,
    );

    // Build unwrap instruction
    const ix = createUnwrapInstruction(
        ctokenAta,
        destination,
        owner.publicKey,
        mint,
        unwrapAmount,
        resolvedSplInterfaceInfo,
        mintInfo.decimals,
        payer.publicKey,
    );

    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [owner]);

    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 200_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );

    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);

    return txId;
}
