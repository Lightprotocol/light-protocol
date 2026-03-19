import {
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionInstruction,
    TransactionSignature,
} from '@solana/web3.js';
import {
    Rpc,
    assertBetaEnabled,
    LIGHT_TOKEN_PROGRAM_ID,
} from '@lightprotocol/stateless.js';
import BN from 'bn.js';
import {
    transferInterface,
    createTransferInterfaceInstructions,
} from './transfer-interface';

/**
 * Transfer tokens from an ATA as an approved delegate.
 *
 * Supports light-token, SPL, and Token-2022 mints. Convenience wrapper
 * around {@link transferInterface} that makes the delegate-transfer API
 * explicit: the delegate is a {@link Signer} (authority), the owner is a
 * {@link PublicKey} (used only for ATA derivation, does NOT sign).
 *
 * @param rpc            RPC connection
 * @param payer          Fee payer (signer)
 * @param source         Source ATA (owner's account)
 * @param mint           Mint address
 * @param recipient      Recipient wallet address (ATA derived + created internally)
 * @param delegate       Delegate authority (signer)
 * @param owner          Owner of the source ATA (does not sign)
 * @param amount         Amount to transfer (must be within approved allowance)
 * @param confirmOptions Optional confirm options
 * @param programId      Token program ID (default: LIGHT_TOKEN_PROGRAM_ID)
 * @returns Transaction signature
 */
export async function transferDelegatedInterface(
    rpc: Rpc,
    payer: Signer,
    source: PublicKey,
    mint: PublicKey,
    recipient: PublicKey,
    delegate: Signer,
    owner: PublicKey,
    amount: number | bigint | BN,
    confirmOptions?: ConfirmOptions,
    programId: PublicKey = LIGHT_TOKEN_PROGRAM_ID,
): Promise<TransactionSignature> {
    assertBetaEnabled();

    return transferInterface(
        rpc,
        payer,
        source,
        mint,
        recipient,
        delegate,
        amount,
        programId,
        confirmOptions,
        { owner },
    );
}

/**
 * Build instruction batches for a delegated transfer on an ATA.
 *
 * Supports light-token, SPL, and Token-2022 mints.
 * Returns `TransactionInstruction[][]`. Send [0..n-2] in parallel, then [n-1].
 *
 * @param rpc         RPC connection
 * @param payer       Fee payer public key
 * @param mint        Mint address
 * @param amount      Amount to transfer
 * @param delegate    Delegate public key (authority)
 * @param owner       Owner of the source ATA (for derivation)
 * @param recipient   Recipient wallet address (ATA derived + created internally)
 * @param decimals    Token decimals
 * @param programId   Token program ID (default: LIGHT_TOKEN_PROGRAM_ID)
 * @returns Instruction batches
 */
export async function createTransferDelegatedInterfaceInstructions(
    rpc: Rpc,
    payer: PublicKey,
    mint: PublicKey,
    amount: number | bigint | BN,
    delegate: PublicKey,
    owner: PublicKey,
    recipient: PublicKey,
    decimals: number,
    programId: PublicKey = LIGHT_TOKEN_PROGRAM_ID,
): Promise<TransactionInstruction[][]> {
    assertBetaEnabled();

    return createTransferInterfaceInstructions(
        rpc,
        payer,
        mint,
        amount,
        delegate,
        recipient,
        decimals,
        { owner, programId },
    );
}
