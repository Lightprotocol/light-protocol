import { CTOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import { ASSOCIATED_TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { PublicKey } from '@solana/web3.js';

export * from './get-token-pool-infos';
export * from './select-input-accounts';
export * from './pack-compressed-token-accounts';
export * from './validation';

/**
 * Get the associated token program ID for a given program ID across SPL, Token-2022, and CToken.
 * @param programId The program ID
 * @returns         The associated token program ID
 */
export function getAtaProgramId(programId: PublicKey) {
    return programId.equals(CTOKEN_PROGRAM_ID)
        ? CTOKEN_PROGRAM_ID
        : ASSOCIATED_TOKEN_PROGRAM_ID;
}
