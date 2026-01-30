import {
    ParsedTokenAccount,
    TreeType,
    assertBetaEnabled,
} from '@lightprotocol/stateless.js';

// Re-export for convenience
export { assertBetaEnabled };

/**
 * Throws if any V1 compressed accounts are present.
 * v3 interface only supports V2 trees.
 */
export function assertV2Only(accounts: ParsedTokenAccount[]): void {
    const v1Count = accounts.filter(
        acc => acc.compressedAccount.treeInfo.treeType === TreeType.StateV1,
    ).length;

    if (v1Count > 0) {
        throw new Error(
            'v3 interface does not support V1 compressed accounts. ' +
                `Found ${v1Count} V1 account(s). ` +
                'Use the main SDK actions (transfer, decompress, merge) to migrate V1 accounts to V2.',
        );
    }
}
