import {
    AccountMeta,
    PublicKey,
    SystemProgram,
    TransactionInstruction,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { Buffer } from 'buffer';
import {
    COMPRESSED_TOKEN_PROGRAM_ID,
    deriveCpiAuthorityPda,
    deriveSplInterfacePdaWithIndex,
} from '../constants';

const CREATE_SPL_INTERFACE_DISCRIMINATOR = Buffer.from([
    23, 169, 27, 122, 147, 169, 209, 152,
]);

interface CreateSplInterfaceAccountsLayoutParams {
    feePayer: PublicKey;
    mint: PublicKey;
    splInterfacePda: PublicKey;
    tokenProgramId: PublicKey;
    cpiAuthorityPda: PublicKey;
}

function createSplInterfaceAccountsLayout({
    feePayer,
    mint,
    splInterfacePda,
    tokenProgramId,
    cpiAuthorityPda,
}: CreateSplInterfaceAccountsLayoutParams): AccountMeta[] {
    return [
        { pubkey: feePayer, isSigner: true, isWritable: true },
        { pubkey: splInterfacePda, isSigner: false, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        { pubkey: mint, isSigner: false, isWritable: true },
        { pubkey: tokenProgramId, isSigner: false, isWritable: false },
        { pubkey: cpiAuthorityPda, isSigner: false, isWritable: false },
    ];
}

export interface CreateSplInterfaceInstructionInput {
    feePayer: PublicKey;
    mint: PublicKey;
    index: number;
    tokenProgramId?: PublicKey;
}

/**
 * Create SPL interface (omnibus account) for an existing SPL mint.
 *
 * @param input                SPL interface instruction input.
 * @param input.feePayer       Fee payer.
 * @param input.mint           SPL mint address.
 * @param input.index          SPL interface derivation index (single required index).
 * @param input.tokenProgramId Token program id (defaults to TOKEN_PROGRAM_ID).
 * @returns Create SPL interface instruction.
 */
export function createSplInterfaceInstruction({
    feePayer,
    mint,
    index,
    tokenProgramId = TOKEN_PROGRAM_ID,
}: CreateSplInterfaceInstructionInput): TransactionInstruction {
    if (!Number.isInteger(index) || index < 0 || index > 255) {
        throw new Error(
            `Invalid SPL interface index ${index}. Expected integer in [0, 255].`,
        );
    }

    const [splInterfacePda] = deriveSplInterfacePdaWithIndex(mint, index);
    const keys = createSplInterfaceAccountsLayout({
        feePayer,
        mint,
        splInterfacePda,
        tokenProgramId,
        cpiAuthorityPda: deriveCpiAuthorityPda(),
    });

    return new TransactionInstruction({
        programId: COMPRESSED_TOKEN_PROGRAM_ID,
        keys,
        data: CREATE_SPL_INTERFACE_DISCRIMINATOR,
    });
}
