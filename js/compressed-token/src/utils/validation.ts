import { ParsedTokenAccount } from '@lightprotocol/stateless.js';
import { PublicKey } from '@solana/web3.js';

/**
 * Check if all input accounts belong to the same mint.
 *
 * @param compressedTokenAccounts   The compressed token accounts
 * @param mint                      The mint of the token pool
 * @returns True if all input accounts belong to the same mint
 */
export function checkMint(
    compressedTokenAccounts: ParsedTokenAccount[],
    mint: PublicKey,
): boolean {
    if (
        !compressedTokenAccounts.every(account =>
            account.parsed.mint.equals(mint),
        )
    ) {
        throw new Error(`All input accounts must belong to the same mint`);
    }

    return true;
}
