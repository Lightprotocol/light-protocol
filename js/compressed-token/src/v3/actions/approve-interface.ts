import {
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
    assertBetaEnabled,
    LIGHT_TOKEN_PROGRAM_ID,
} from '@lightprotocol/stateless.js';
import BN from 'bn.js';
import {
    createApproveInterfaceInstructions,
    createRevokeInterfaceInstructions,
    type ApproveRevokeOptions,
} from '../instructions/approve-interface';
import { getAssociatedTokenAddressInterface } from '../get-associated-token-address-interface';
import { getMintInterface } from '../get-mint-interface';
import { sliceLast } from './slice-last';
import { TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID } from '@solana/spl-token';

/**
 * Approve a delegate for an associated token account.
 *
 * Supports light-token, SPL, and Token-2022 mints. For light-token mints,
 * loads cold accounts if needed before sending the approve instruction.
 *
 * @remarks For light-token mints, all cold (compressed) balances are loaded
 * into the hot ATA, not just the delegation amount. The `amount` parameter
 * only controls the delegate's spending limit.
 *
 * @param rpc            RPC connection
 * @param payer          Fee payer (signer)
 * @param tokenAccount   ATA address
 * @param mint           Mint address
 * @param delegate       Delegate to approve
 * @param amount         Amount to delegate
 * @param owner          Owner of the token account (signer)
 * @param confirmOptions Optional confirm options
 * @param programId      Token program ID (default: LIGHT_TOKEN_PROGRAM_ID)
 * @param wrap           When true and mint is SPL/T22, wrap into light-token then approve
 * @returns Transaction signature
 */
export async function approveInterface(
    rpc: Rpc,
    payer: Signer,
    tokenAccount: PublicKey,
    mint: PublicKey,
    delegate: PublicKey,
    amount: number | bigint | BN,
    owner: Signer,
    confirmOptions?: ConfirmOptions,
    programId: PublicKey = LIGHT_TOKEN_PROGRAM_ID,
    wrap = false,
    options?: ApproveRevokeOptions,
    decimals?: number,
): Promise<TransactionSignature> {
    assertBetaEnabled();

    const expectedAta = getAssociatedTokenAddressInterface(
        mint,
        owner.publicKey,
        false,
        programId,
    );
    if (!tokenAccount.equals(expectedAta)) {
        throw new Error(
            `Token account mismatch. Expected ${expectedAta.toBase58()}, got ${tokenAccount.toBase58()}`,
        );
    }

    const isSplOrT22 =
        programId.equals(TOKEN_PROGRAM_ID) ||
        programId.equals(TOKEN_2022_PROGRAM_ID);
    const resolvedDecimals =
        decimals ??
        (isSplOrT22 && !wrap ? 0 : (await getMintInterface(rpc, mint)).mint.decimals);
    const batches = await createApproveInterfaceInstructions(
        rpc,
        payer.publicKey,
        mint,
        tokenAccount,
        delegate,
        amount,
        owner.publicKey,
        resolvedDecimals,
        programId,
        wrap,
        options,
    );

    const additionalSigners = dedupeSigner(payer, [owner]);
    const { rest: loads, last: approveIxs } = sliceLast(batches);

    await Promise.all(
        loads.map(async ixs => {
            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(
                ixs,
                payer,
                blockhash,
                additionalSigners,
            );
            return sendAndConfirmTx(rpc, tx, confirmOptions);
        }),
    );

    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(
        approveIxs,
        payer,
        blockhash,
        additionalSigners,
    );
    return sendAndConfirmTx(rpc, tx, confirmOptions);
}

/**
 * Revoke delegation for an associated token account.
 *
 * Supports light-token, SPL, and Token-2022 mints. For light-token mints,
 * loads cold accounts if needed before sending the revoke instruction.
 *
 * @remarks For light-token mints, all cold (compressed) balances are loaded
 * into the hot ATA before the revoke instruction.
 *
 * @param rpc            RPC connection
 * @param payer          Fee payer (signer)
 * @param tokenAccount   ATA address
 * @param mint           Mint address
 * @param owner          Owner of the token account (signer)
 * @param confirmOptions Optional confirm options
 * @param programId      Token program ID (default: LIGHT_TOKEN_PROGRAM_ID)
 * @param wrap           When true and mint is SPL/T22, wrap into light-token then revoke
 * @returns Transaction signature
 */
export async function revokeInterface(
    rpc: Rpc,
    payer: Signer,
    tokenAccount: PublicKey,
    mint: PublicKey,
    owner: Signer,
    confirmOptions?: ConfirmOptions,
    programId: PublicKey = LIGHT_TOKEN_PROGRAM_ID,
    wrap = false,
    options?: ApproveRevokeOptions,
    decimals?: number,
): Promise<TransactionSignature> {
    assertBetaEnabled();

    const expectedAta = getAssociatedTokenAddressInterface(
        mint,
        owner.publicKey,
        false,
        programId,
    );
    if (!tokenAccount.equals(expectedAta)) {
        throw new Error(
            `Token account mismatch. Expected ${expectedAta.toBase58()}, got ${tokenAccount.toBase58()}`,
        );
    }

    const isSplOrT22 =
        programId.equals(TOKEN_PROGRAM_ID) ||
        programId.equals(TOKEN_2022_PROGRAM_ID);
    const resolvedDecimals =
        decimals ??
        (isSplOrT22 && !wrap ? 0 : (await getMintInterface(rpc, mint)).mint.decimals);
    const batches = await createRevokeInterfaceInstructions(
        rpc,
        payer.publicKey,
        mint,
        tokenAccount,
        owner.publicKey,
        resolvedDecimals,
        programId,
        wrap,
        options,
    );

    const additionalSigners = dedupeSigner(payer, [owner]);
    const { rest: loads, last: revokeIxs } = sliceLast(batches);

    await Promise.all(
        loads.map(async ixs => {
            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(
                ixs,
                payer,
                blockhash,
                additionalSigners,
            );
            return sendAndConfirmTx(rpc, tx, confirmOptions);
        }),
    );

    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(revokeIxs, payer, blockhash, additionalSigners);
    return sendAndConfirmTx(rpc, tx, confirmOptions);
}
