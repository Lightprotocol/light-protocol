import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import {
    MerkleContext,
    ValidityProofWithContext,
} from '@lightprotocol/stateless.js';
import { createMintToInstruction } from './mint-to';
import { createMintToCompressedInstruction } from './mint-to-compressed';

/**
 * Create mint-to instruction for compressed mints
 * For SPL mints, use CompressedTokenProgram.mintTo() instead
 *
 * @param mint - Compressed mint address
 * @param authority - Mint authority
 * @param payer - Transaction fee payer
 * @param recipient - Recipient address(es) - account address or owner pubkey
 * @param amount - Amount(s) to mint
 * @param validityProof - Validity proof for the mint
 * @param merkleContext - Merkle context of the mint
 * @param mintData - Current mint data
 * @param outputQueue - Output queue for the minted accounts
 * @param tokensOutQueue - Tokens output queue
 * @param tokenAccountVersion - Token account version (default: 3)
 *
 * @returns Transaction instruction
 */
export function createMintToInterfaceInstruction(
    mint: PublicKey,
    authority: PublicKey,
    payer: PublicKey,
    recipient: PublicKey | PublicKey[],
    amount: number | bigint | Array<number | bigint>,
    validityProof: ValidityProofWithContext,
    merkleContext: MerkleContext,
    mintData: {
        supply: bigint;
        decimals: number;
        mintAuthority: PublicKey | null;
        freezeAuthority: PublicKey | null;
        splMint: PublicKey;
        splMintInitialized: boolean;
        version: number;
        metadata?: {
            updateAuthority: PublicKey | null;
            name: string;
            symbol: string;
            uri: string;
        };
    },
    outputQueue: PublicKey,
    tokensOutQueue: PublicKey,
    tokenAccountVersion: number = 3,
): TransactionInstruction {
    // Check if recipient is an array or single value
    if (Array.isArray(recipient)) {
        // Multiple recipients - use mintToCompressed
        if (!Array.isArray(amount)) {
            throw new Error(
                'Amount must be an array when recipient is an array',
            );
        }
        if (recipient.length !== amount.length) {
            throw new Error(
                'Recipient and amount arrays must have the same length',
            );
        }

        const recipients = recipient.map((r, i) => ({
            recipient: r,
            amount: amount[i],
        }));

        return createMintToCompressedInstruction(
            mint,
            authority,
            payer,
            validityProof,
            merkleContext,
            mintData,
            outputQueue,
            tokensOutQueue,
            recipients,
            tokenAccountVersion,
        );
    } else {
        // Single recipient - use mintTo
        if (Array.isArray(amount)) {
            throw new Error(
                'Amount must be a single value when recipient is a single address',
            );
        }

        return createMintToInstruction(
            mint,
            authority,
            payer,
            validityProof,
            merkleContext,
            mintData,
            outputQueue,
            tokensOutQueue,
            recipient,
            amount,
        );
    }
}
