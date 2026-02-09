import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import {
    getOutputTreeInfo,
    ValidityProofWithContext,
} from '@lightprotocol/stateless.js';
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
 * Create mint-to instruction for SPL, Token-2022, or compressed token mints.
 * This instruction ONLY mints to decompressed/onchain token accounts.
 *
 * @param mintInterface   Mint interface (SPL, Token-2022, or compressed).
 * @param destination     Destination onchain token account address.
 * @param authority       Mint authority public key.
 * @param payer           Fee payer public key.
 * @param amount          Amount to mint.
 * @param validityProof   Validity proof (required for light mints).
 * @param multiSigners    Multi-signature signer public keys.
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

    // SPL/T22
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

    if (!validityProof) {
        throw new Error('Validity proof required for c-token mint-to');
    }
    if (!mintInterface.mintContext) {
        throw new Error('mintContext required for c-token mint-to');
    }

    const mintData = {
        supply: mintInterface.mint.supply,
        decimals: mintInterface.mint.decimals,
        mintAuthority: mintInterface.mint.mintAuthority,
        freezeAuthority: mintInterface.mint.freezeAuthority,
        splMint: mintInterface.mintContext.splMint,
        cmintDecompressed: mintInterface.mintContext.cmintDecompressed,
        version: mintInterface.mintContext.version,
        mintSigner: mintInterface.mintContext.mintSigner,
        bump: mintInterface.mintContext.bump,
        metadata: mintInterface.tokenMetadata
            ? {
                  updateAuthority:
                      mintInterface.tokenMetadata.updateAuthority || null,
                  name: mintInterface.tokenMetadata.name,
                  symbol: mintInterface.tokenMetadata.symbol,
                  uri: mintInterface.tokenMetadata.uri,
              }
            : undefined,
    };

    return createCtokenMintToInstruction(
        authority,
        payer,
        validityProof,
        mintInterface.merkleContext,
        mintData,
        getOutputTreeInfo(mintInterface.merkleContext),
        destination,
        amount,
    );
}
