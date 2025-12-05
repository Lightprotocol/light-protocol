import { PublicKey, Signer, TransactionInstruction } from '@solana/web3.js';
import { CTOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import {
    TOKEN_2022_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    createTransferInstruction as createSplTransferInstruction,
} from '@solana/spl-token';

/**
 * CToken Transfer discriminator (matches InstructionType::CTokenTransfer = 3)
 */
const CTOKEN_TRANSFER_DISCRIMINATOR = 3;

/**
 * Create a CToken transfer instruction for hot (on-chain) accounts.
 * Uses CTokenTransfer instruction (discriminator 3) which wraps SPL Token transfer.
 *
 * Accounts:
 * 1. source (mutable) - Source CToken account
 * 2. destination (mutable) - Destination CToken account
 * 3. authority (signer) - Owner of source account
 * 4. payer (optional, signer, mutable) - For compressible extension top-up
 *
 * @param source        Source CToken account
 * @param destination   Destination CToken account
 * @param owner         Owner of the source account (signer)
 * @param amount        Amount to transfer
 * @param payer         Optional payer for compressible extension top-up
 * @returns TransactionInstruction for CToken transfer
 */
export function createCTokenTransferInstruction(
    source: PublicKey,
    destination: PublicKey,
    owner: PublicKey,
    amount: number | bigint,
    payer?: PublicKey,
): TransactionInstruction {
    // Instruction data format (from CTOKEN_TRANSFER.md):
    // byte 0: discriminator (3)
    // byte 1: padding (0)
    // bytes 2-9: amount (u64 LE) - SPL TokenInstruction::Transfer format
    const data = Buffer.alloc(10);
    data.writeUInt8(CTOKEN_TRANSFER_DISCRIMINATOR, 0);
    data.writeUInt8(0, 1); // padding
    data.writeBigUInt64LE(BigInt(amount), 2);

    const keys = [
        { pubkey: source, isSigner: false, isWritable: true },
        { pubkey: destination, isSigner: false, isWritable: true },
        { pubkey: owner, isSigner: true, isWritable: false },
    ];

    // Add payer as 4th account if provided and different from owner
    // (for compressible extension top-up)
    if (payer && !payer.equals(owner)) {
        keys.push({ pubkey: payer, isSigner: true, isWritable: true });
    }

    return new TransactionInstruction({
        programId: CTOKEN_PROGRAM_ID,
        keys,
        data,
    });
}

/**
 * Construct a transfer instruction for SPL Token, Token-2022, or CToken (hot accounts).
 * Matches SPL Token createTransferInstruction signature exactly.
 * Defaults to CToken program.
 *
 * Dispatches to the appropriate program based on `programId`:
 * - `CTOKEN_PROGRAM_ID` -> CToken hot-to-hot transfer (default)
 * - `TOKEN_PROGRAM_ID` -> SPL Token transfer
 * - `TOKEN_2022_PROGRAM_ID` -> Token-2022 transfer
 *
 * Note: This is for on-chain (hot) token accounts only.
 * For compressed (cold) token transfers, use the `transfer` action.
 * For cross-program transfers (SPL <> CToken), use `wrap`/`unwrap`.
 *
 * @param source        Source token account
 * @param destination   Destination token account
 * @param owner         Owner of the source account
 * @param amount        Amount to transfer
 * @param multiSigners  Signing accounts if `owner` is a multisig (SPL only)
 * @param programId     Token program ID (default: CTOKEN_PROGRAM_ID)
 * @param payer         Fee payer for compressible top-up (CToken only)
 *
 * @example
 * // CToken hot transfer (default) - same signature as SPL!
 * const ix = createTransferInterfaceInstruction(
 *     sourceCtokenAccount,
 *     destCtokenAccount,
 *     owner,
 *     1000000n,
 * );
 *
 * @example
 * // SPL Token transfer - identical call, just change programId
 * import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
 * const ix = createTransferInterfaceInstruction(
 *     sourceAta,
 *     destAta,
 *     owner,
 *     1000000n,
 *     [],
 *     TOKEN_PROGRAM_ID,
 * );
 */
export function createTransferInterfaceInstruction(
    source: PublicKey,
    destination: PublicKey,
    owner: PublicKey,
    amount: number | bigint,
    multiSigners: (Signer | PublicKey)[] = [],
    programId: PublicKey = CTOKEN_PROGRAM_ID,
    payer?: PublicKey,
): TransactionInstruction {
    if (programId.equals(CTOKEN_PROGRAM_ID)) {
        if (multiSigners.length > 0) {
            throw new Error(
                'CToken transfer does not support multi-signers. Use a single owner.',
            );
        }
        return createCTokenTransferInstruction(
            source,
            destination,
            owner,
            amount,
            payer,
        );
    }

    if (
        programId.equals(TOKEN_PROGRAM_ID) ||
        programId.equals(TOKEN_2022_PROGRAM_ID)
    ) {
        return createSplTransferInstruction(
            source,
            destination,
            owner,
            amount,
            multiSigners.map(pk =>
                pk instanceof PublicKey ? pk : pk.publicKey,
            ),
            programId,
        );
    }

    throw new Error(`Unsupported program ID: ${programId.toBase58()}`);
}
