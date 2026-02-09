import { PublicKey } from '@solana/web3.js';
import { getAssociatedTokenAddressSync } from '@solana/spl-token';
import { CTOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import { getAtaProgramId } from './ata-utils';

/**
 * Derive the canonical associated token address for any of SPL/T22/light-token.
 * Defaults to using light-token as the canonical ATA.
 *
 * @param mint                      Mint public key
 * @param owner                     Owner public key
 * @param allowOwnerOffCurve        Allow owner to be a PDA. Default false.
 * @param programId                 Token program ID. Default light-token.
 *
 * @param associatedTokenProgramId  Associated token program ID. Default
 *                                  auto-detected.
 * @returns                         Associated token address.
 */
export function getAssociatedTokenAddressInterface(
    mint: PublicKey,
    owner: PublicKey,
    allowOwnerOffCurve = false,
    programId: PublicKey = CTOKEN_PROGRAM_ID,
    associatedTokenProgramId?: PublicKey,
): PublicKey {
    const effectiveAssociatedProgramId =
        associatedTokenProgramId ?? getAtaProgramId(programId);

    // by passing program id, user indicates preference for the canonical ATA.
    return getAssociatedTokenAddressSync(
        mint,
        owner,
        allowOwnerOffCurve,
        programId,
        effectiveAssociatedProgramId,
    );
}
