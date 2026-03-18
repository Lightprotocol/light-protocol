/**
 * MintAction instruction builder.
 */

import type { Address } from '@solana/addresses';
import {
    AccountRole,
    type Instruction,
    type AccountMeta,
} from '@solana/instructions';

import {
    LIGHT_TOKEN_PROGRAM_ID,
    LIGHT_SYSTEM_PROGRAM_ID,
    CPI_AUTHORITY,
    REGISTERED_PROGRAM_PDA,
    ACCOUNT_COMPRESSION_AUTHORITY_PDA,
    ACCOUNT_COMPRESSION_PROGRAM_ID,
    SYSTEM_PROGRAM_ID,
} from '../constants.js';

import { encodeMintActionInstructionData } from '../codecs/mint-action.js';
import type { MintActionInstructionData } from '../codecs/mint-action.js';

// ============================================================================
// MINT ACTION INSTRUCTION
// ============================================================================

/**
 * CPI context accounts for mint action.
 */
export interface MintActionCpiContextAccounts {
    /** Fee payer (writable signer) */
    feePayer: Address;
    /** CPI authority PDA (readonly) */
    cpiAuthorityPda: Address;
    /** CPI context account (writable) */
    cpiContext: Address;
}

/**
 * Parameters for MintAction instruction.
 */
export interface MintActionParams {
    /** Mint signer (optional, role depends on whether createMint is set) */
    mintSigner?: Address;
    /** Authority (signer) - mint authority for the token */
    authority: Address;
    /** Fee payer (signer, writable) */
    feePayer: Address;
    /** Output queue (writable) */
    outOutputQueue: Address;
    /** Merkle tree (writable) */
    merkleTree: Address;
    /** Structured instruction data (encoded via codec) */
    data: MintActionInstructionData;
    /** Packed remaining accounts (optional) */
    packedAccounts?: AccountMeta[];
    /** Compressible config account (optional, readonly) */
    compressibleConfig?: Address;
    /** Compressed mint account (optional, writable) */
    cmint?: Address;
    /** Rent sponsor (optional, writable) */
    rentSponsor?: Address;
    /** CPI context accounts (optional, triggers CPI context path) */
    cpiContextAccounts?: MintActionCpiContextAccounts;
}

/**
 * Creates a MintAction instruction (discriminator: 103).
 *
 * MintAction supports batch minting operations for compressed tokens.
 *
 * Normal path account layout (matches on-chain program parsing order):
 * 0: light_system_program (readonly)
 * 1: authority (readonly signer)
 * [optional: mintSigner — only when createMint is set]
 * [optional: compressibleConfig — for DecompressMint/CompressAndCloseMint]
 * [optional: cmint — for decompressed mints]
 * [optional: rentSponsor — for DecompressMint/CompressAndCloseMint]
 * N: fee_payer (writable signer)        \
 * N+1: cpi_authority_pda (readonly)      |
 * N+2: registered_program_pda (readonly) | LightSystemAccounts (6)
 * N+3: account_compression_authority     |
 * N+4: account_compression_program       |
 * N+5: system_program (readonly)        /
 * N+6: out_output_queue (writable)
 * N+7: merkle_tree (writable)
 * [...packed_accounts]
 *
 * CPI context path account layout:
 * 0: light_system_program (readonly)
 * 1: authority (readonly signer)
 * 2: fee_payer (writable signer)
 * 3: cpi_authority_pda (readonly)
 * 4: cpi_context (writable)
 *
 * @param params - MintAction parameters
 * @returns The MintAction instruction
 */
export function createMintActionInstruction(
    params: MintActionParams,
): Instruction {
    const {
        mintSigner,
        authority,
        feePayer,
        outOutputQueue,
        merkleTree,
        data: mintActionData,
        packedAccounts,
        compressibleConfig,
        cmint,
        rentSponsor,
        cpiContextAccounts,
    } = params;

    const accounts: AccountMeta[] = [];

    if (cpiContextAccounts) {
        // CPI context path
        accounts.push(
            { address: LIGHT_SYSTEM_PROGRAM_ID, role: AccountRole.READONLY },
            { address: authority, role: AccountRole.READONLY_SIGNER },
            { address: cpiContextAccounts.feePayer, role: AccountRole.WRITABLE_SIGNER },
            { address: cpiContextAccounts.cpiAuthorityPda, role: AccountRole.READONLY },
            { address: cpiContextAccounts.cpiContext, role: AccountRole.WRITABLE },
        );
    } else {
        // Normal path: program parses optional accounts between authority
        // and Light system accounts (fee_payer, cpi_authority, etc.)
        accounts.push(
            { address: LIGHT_SYSTEM_PROGRAM_ID, role: AccountRole.READONLY },
            { address: authority, role: AccountRole.READONLY_SIGNER },
        );

        // Optional accounts — order must match on-chain parsing:
        // mint_signer → compressible_config → cmint → rent_sponsor
        if (mintSigner) {
            const hasCreateMint = mintActionData.createMint !== null;
            accounts.push({
                address: mintSigner,
                role: hasCreateMint
                    ? AccountRole.READONLY_SIGNER
                    : AccountRole.READONLY,
            });
        }
        if (compressibleConfig) {
            accounts.push({ address: compressibleConfig, role: AccountRole.READONLY });
        }
        if (cmint) {
            accounts.push({ address: cmint, role: AccountRole.WRITABLE });
        }
        if (rentSponsor) {
            accounts.push({ address: rentSponsor, role: AccountRole.WRITABLE });
        }

        // Light system accounts
        accounts.push(
            { address: feePayer, role: AccountRole.WRITABLE_SIGNER },
            { address: CPI_AUTHORITY, role: AccountRole.READONLY },
            { address: REGISTERED_PROGRAM_PDA, role: AccountRole.READONLY },
            {
                address: ACCOUNT_COMPRESSION_AUTHORITY_PDA,
                role: AccountRole.READONLY,
            },
            {
                address: ACCOUNT_COMPRESSION_PROGRAM_ID,
                role: AccountRole.READONLY,
            },
            { address: SYSTEM_PROGRAM_ID, role: AccountRole.READONLY },
            { address: outOutputQueue, role: AccountRole.WRITABLE },
            { address: merkleTree, role: AccountRole.WRITABLE },
        );
    }

    // Add packed remaining accounts
    if (packedAccounts) {
        accounts.push(...packedAccounts);
    }

    // Encode instruction data via codec (includes discriminator)
    const data = encodeMintActionInstructionData(mintActionData);

    return {
        programAddress: LIGHT_TOKEN_PROGRAM_ID,
        accounts,
        data,
    };
}
