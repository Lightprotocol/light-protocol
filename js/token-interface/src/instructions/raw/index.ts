import {
    createAssociatedTokenAccountInterfaceIdempotentInstruction,
    createLightTokenApproveInstruction,
    createLightTokenFreezeAccountInstruction,
    createLightTokenRevokeInstruction,
    createLightTokenThawAccountInstruction,
    createLightTokenTransferCheckedInstruction,
} from '@lightprotocol/compressed-token';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import type { TransactionInstruction } from '@solana/web3.js';
import { createLoadInstructionInternal } from '../../load';
import { getAtaAddress } from '../../read';
import type {
    CreateFreezeInstructionsInput,
    CreateRawApproveInstructionInput,
    CreateRawAtaInstructionInput,
    CreateRawLoadInstructionInput,
    CreateRawRevokeInstructionInput,
    CreateRawTransferInstructionInput,
    CreateThawInstructionsInput,
} from '../../types';

export function createAtaInstruction({
    payer,
    owner,
    mint,
    programId,
}: CreateRawAtaInstructionInput): TransactionInstruction {
    const targetProgramId = programId ?? LIGHT_TOKEN_PROGRAM_ID;
    const associatedToken = getAtaAddress({
        owner,
        mint,
        programId: targetProgramId,
    });

    return createAssociatedTokenAccountInterfaceIdempotentInstruction(
        payer,
        associatedToken,
        owner,
        mint,
        targetProgramId,
    );
}

export async function createLoadInstruction({
    rpc,
    payer,
    owner,
    mint,
}: CreateRawLoadInstructionInput): Promise<TransactionInstruction | null> {
    const load = await createLoadInstructionInternal({
        rpc,
        payer,
        owner,
        mint,
    });

    return load?.instructions[load.instructions.length - 1] ?? null;
}

export function createTransferCheckedInstruction({
    source,
    destination,
    mint,
    authority,
    payer,
    amount,
    decimals,
}: CreateRawTransferInstructionInput): TransactionInstruction {
    return createLightTokenTransferCheckedInstruction(
        source,
        destination,
        mint,
        authority,
        amount,
        decimals,
        payer,
    );
}

export const createTransferInstruction = createTransferCheckedInstruction;
export const getTransferInstruction = createTransferCheckedInstruction;
export const getLoadInstruction = createLoadInstruction;
export const getCreateAtaInstruction = createAtaInstruction;

export function createApproveInstruction({
    tokenAccount,
    delegate,
    owner,
    amount,
    payer,
}: CreateRawApproveInstructionInput): TransactionInstruction {
    return createLightTokenApproveInstruction(
        tokenAccount,
        delegate,
        owner,
        amount,
        payer,
    );
}

export function createRevokeInstruction({
    tokenAccount,
    owner,
    payer,
}: CreateRawRevokeInstructionInput): TransactionInstruction {
    return createLightTokenRevokeInstruction(tokenAccount, owner, payer);
}

export function createFreezeInstruction({
    tokenAccount,
    mint,
    freezeAuthority,
}: CreateFreezeInstructionsInput): TransactionInstruction {
    return createLightTokenFreezeAccountInstruction(
        tokenAccount,
        mint,
        freezeAuthority,
    );
}

export function createThawInstruction({
    tokenAccount,
    mint,
    freezeAuthority,
}: CreateThawInstructionsInput): TransactionInstruction {
    return createLightTokenThawAccountInstruction(
        tokenAccount,
        mint,
        freezeAuthority,
    );
}
