import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { ValidityProofWithContext } from '@lightprotocol/stateless.js';
import { createMintToInstruction as createSplMintToInstruction } from '@solana/spl-token';
import { createMintToInstruction as createCtokenMintToInstruction } from './mint-to';
import { MintInterface } from '../get-mint-interface';

// Keep old interface type for backwards compatibility export
export interface CreateMintToInterfaceInstructionParams {
    mintInterface: MintInterface;
    destination: PublicKey;
    authority: PublicKey;
    payer: PublicKey;
    amount: number | bigint;
    validityProof?: ValidityProofWithContext;
    multiSigners?: PublicKey[];
}

/**
 * Create mint-to instruction for SPL, Token-2022, or light-token mints.
 * This instruction ONLY mints to light-token associated token accounts (hot).
 *
 * For light-token mints, the light mint account must exist (mint must be decompressed first).
 *
 * @param mintInterface   Mint interface (SPL, Token-2022, or light-token).
 * @param destination     Destination onchain token account address.
 * @param authority       Mint authority public key.
 * @param payer           Fee payer public key.
 * @param amount          Amount to mint.
 * @param validityProof   Not used (legacy parameter, kept for compatibility).
 * @param multiSigners    Multi-signature signer public keys (SPL/T22 only).
 */
export function createMintToInterfaceInstruction(
    mintInterface: MintInterface,
    destination: PublicKey,
    authority: PublicKey,
    payer: PublicKey,
    amount: number | bigint,
    validityProof?: ValidityProofWithContext,
    multiSigners: PublicKey[] = [],
): TransactionInstruction {
    const mint = mintInterface.mint.address;
    const programId = mintInterface.programId;

    // SPL/T22 - no merkleContext means it's a native SPL mint
    if (!mintInterface.merkleContext) {
        return createSplMintToInstruction(
            mint,
            destination,
            authority,
            BigInt(amount.toString()),
            multiSigners,
            programId,
        );
    }

    // light-token (light-token) - use simple CTokenMintTo instruction
    // The light mint account must exist for this to work (mint must be decompressed first)
    if (!mintInterface.mintContext) {
        throw new Error('mintContext required for light-token mint-to');
    }

    // Use payer as fee payer for top-ups if different from authority
    const feePayer = authority.equals(payer) ? undefined : payer;

    return createCtokenMintToInstruction({
        mint,
        destination,
        amount,
        authority,
        feePayer,
    });
}
