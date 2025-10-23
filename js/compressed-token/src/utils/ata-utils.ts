import {
    ASSOCIATED_TOKEN_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
} from '@solana/spl-token';
import { CTOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import { PublicKey } from '@solana/web3.js';

/**
 * Get the appropriate ATA program ID for a given token program ID
 * @param tokenProgramId - The token program ID
 * @returns The associated token program ID
 */
export function getAtaProgramId(tokenProgramId: PublicKey): PublicKey {
    if (tokenProgramId.equals(CTOKEN_PROGRAM_ID)) {
        return CTOKEN_PROGRAM_ID;
    }
    return ASSOCIATED_TOKEN_PROGRAM_ID;
}

