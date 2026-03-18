import {
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionInstruction,
    TransactionSignature,
} from '@solana/web3.js';
import { Rpc, assertBetaEnabled } from '@lightprotocol/stateless.js';
import BN from 'bn.js';
import { transferInterface } from './transfer-interface';
import { createTransferInterfaceInstructions } from '../instructions/transfer-interface';

/**
 * Transfer tokens from a light-token ATA as an approved delegate.
 *
 * Convenience wrapper around {@link transferInterface} that makes the
 * delegate-transfer API explicit: the delegate is a {@link Signer} (authority),
 * the owner is a {@link PublicKey} (used only for ATA derivation, does NOT
 * sign).
 *
 * @param rpc            RPC connection
 * @param payer          Fee payer (signer)
 * @param source         Source light-token ATA (owner's account)
 * @param mint           Mint address
 * @param destination    Destination light-token ATA
 * @param delegate       Delegate authority (signer)
 * @param owner          Owner of the source ATA (does not sign)
 * @param amount         Amount to transfer (must be within approved allowance)
 * @param confirmOptions Optional confirm options
 * @returns Transaction signature
 */
export async function transferDelegatedInterface(
    rpc: Rpc,
    payer: Signer,
    source: PublicKey,
    mint: PublicKey,
    destination: PublicKey,
    delegate: Signer,
    owner: PublicKey,
    amount: number | bigint | BN,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    assertBetaEnabled();

    return transferInterface(
        rpc,
        payer,
        source,
        mint,
        destination,
        delegate,
        amount,
        undefined,
        confirmOptions,
        { owner },
    );
}

/**
 * Build instruction batches for a delegated transfer on a light-token ATA.
 *
 * Returns `TransactionInstruction[][]`. Send [0..n-2] in parallel, then [n-1].
 *
 * @param rpc         RPC connection
 * @param payer       Fee payer public key
 * @param mint        Mint address
 * @param amount      Amount to transfer
 * @param delegate    Delegate public key (authority)
 * @param owner       Owner of the source ATA (for derivation)
 * @param destination Destination ATA address
 * @param decimals    Token decimals
 * @returns Instruction batches
 */
export async function createTransferDelegatedInterfaceInstructions(
    rpc: Rpc,
    payer: PublicKey,
    mint: PublicKey,
    amount: number | bigint | BN,
    delegate: PublicKey,
    owner: PublicKey,
    destination: PublicKey,
    decimals: number,
): Promise<TransactionInstruction[][]> {
    assertBetaEnabled();

    return createTransferInterfaceInstructions(
        rpc,
        payer,
        mint,
        amount,
        delegate,
        destination,
        decimals,
        { owner },
    );
}
