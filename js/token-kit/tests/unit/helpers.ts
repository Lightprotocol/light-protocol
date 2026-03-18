/**
 * Shared test helpers for unit tests.
 */

import { address } from '@solana/addresses';
import { vi } from 'vitest';

import {
    type CompressedTokenAccount,
    type CompressedAccount,
    type TreeInfo,
    type LightIndexer,
    type MintContext,
    type SplInterfaceInfo,
    type BuilderRpc,
    TreeType,
    AccountState,
    SPL_TOKEN_PROGRAM_ID,
} from '../../src/index.js';
import type { DeserializedCompressedMint } from '../../src/codecs/mint-deserialize.js';

// ============================================================================
// CONSTANTS
// ============================================================================

export const MOCK_TREE = address('amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx');
export const MOCK_QUEUE = address('SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7');
export const MOCK_MINT = address('So11111111111111111111111111111111111111112');
export const MOCK_OWNER = address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
export const MOCK_CTOKEN_PROGRAM = address('cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m');
export const MOCK_POOL = address('GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy');
export const MOCK_MINT_SIGNER = address('BPFLoaderUpgradeab1e11111111111111111111111');

// ============================================================================
// EXISTING HELPERS
// ============================================================================

export function createMockTokenAccount(amount: bigint): CompressedTokenAccount {
    const mockTreeInfo: TreeInfo = {
        tree: MOCK_TREE,
        queue: MOCK_QUEUE,
        treeType: TreeType.StateV2,
    };
    const mockAccount: CompressedAccount = {
        hash: new Uint8Array(32),
        address: null,
        owner: MOCK_CTOKEN_PROGRAM,
        lamports: 0n,
        data: null,
        leafIndex: 0,
        treeInfo: mockTreeInfo,
        proveByIndex: false,
        seq: null,
        slotCreated: 0n,
    };
    return {
        token: {
            mint: MOCK_MINT,
            owner: MOCK_OWNER,
            amount,
            delegate: null,
            state: AccountState.Initialized,
            tlv: null,
        },
        account: mockAccount,
    };
}

export function createMockTreeInfo(
    treeType: TreeType,
    nextTree?: TreeInfo,
): TreeInfo {
    return {
        tree: MOCK_TREE,
        queue: MOCK_QUEUE,
        treeType,
        nextTreeInfo: nextTree,
    };
}

// ============================================================================
// MOCK INDEXER
// ============================================================================

/**
 * Creates a mock LightIndexer with all methods stubbed via vi.fn().
 * Pass overrides to mock specific return values.
 */
export function createMockIndexer(
    overrides?: Partial<LightIndexer>,
): LightIndexer {
    return {
        getCompressedAccount: vi.fn(),
        getCompressedAccountByHash: vi.fn(),
        getCompressedTokenAccountsByOwner: vi.fn(),
        getMultipleCompressedAccounts: vi.fn(),
        getValidityProof: vi.fn(),
        getCompressedTokenBalancesByOwner: vi.fn(),
        getCompressedMintTokenHolders: vi.fn(),
        getCompressedTokenAccountBalance: vi.fn(),
        getSignaturesForTokenOwner: vi.fn(),
        ...overrides,
    };
}

// ============================================================================
// MOCK RPC
// ============================================================================

/**
 * Creates a mock BuilderRpc with getAccountInfo stubbed.
 * Default: returns null (account not found).
 */
export function createMockRpc(
    overrides?: Partial<BuilderRpc>,
): BuilderRpc {
    return {
        getAccountInfo: vi.fn().mockResolvedValue({ value: null }),
        ...overrides,
    };
}

/**
 * Creates base64-encoded SPL mint account data (82 bytes) with the given decimals.
 * Returns [base64String, 'base64'] tuple matching the RPC response shape.
 */
export function createBase64MintData(
    decimals: number,
    supply: bigint = 1000000n,
    hasFreezeAuthority = false,
): [string, string] {
    const data = new Uint8Array(82);
    const view = new DataView(data.buffer);
    // mintAuthorityOption = 1
    view.setUint32(0, 1, true);
    // supply at offset 36
    view.setBigUint64(36, supply, true);
    // decimals at offset 44
    data[44] = decimals;
    // isInitialized at offset 45
    data[45] = 1;
    // freezeAuthorityOption at offset 46
    view.setUint32(46, hasFreezeAuthority ? 1 : 0, true);

    const base64 = btoa(String.fromCharCode(...data));
    return [base64, 'base64'];
}

/**
 * Creates a mock RPC that returns valid mint data for getAccountInfo calls.
 */
export function createMockRpcWithMint(
    decimals: number,
    supply: bigint = 1000000n,
): BuilderRpc {
    const mintData = createBase64MintData(decimals, supply);
    return createMockRpc({
        getAccountInfo: vi.fn().mockResolvedValue({
            value: {
                owner: SPL_TOKEN_PROGRAM_ID,
                data: mintData,
            },
        }),
    });
}

// ============================================================================
// MOCK MINT CONTEXT
// ============================================================================

/**
 * Creates a 149-byte compressed mint data Uint8Array.
 * Layout: BaseMint(0-81) + MintContext(82-148).
 */
export function createMockCompressedMintData(
    decimals = 9,
    supply = 1000000n,
): Uint8Array {
    const data = new Uint8Array(149);
    const view = new DataView(data.buffer);
    // BaseMint
    view.setUint32(0, 1, true); // mintAuthorityOption = 1
    data.set(new Uint8Array(32).fill(0x11), 4); // mintAuthority
    view.setBigUint64(36, supply, true); // supply
    data[44] = decimals;
    data[45] = 1; // isInitialized
    view.setUint32(46, 0, true); // freezeAuthorityOption = 0
    // MintContext
    data[82] = 0; // version
    data[83] = 0; // cmintDecompressed = false
    data.set(new Uint8Array(32).fill(0x22), 84); // splMint
    data.set(new Uint8Array(32).fill(0x33), 116); // mintSigner
    data[148] = 254; // bump
    return data;
}

/**
 * Creates a mock MintContext for builders that accept mintContext override.
 * All fields populated with consistent test values.
 */
export function createMockMintContext(
    overrides?: Partial<MintContext>,
): MintContext {
    const mintData = createMockCompressedMintData();
    const mockDeserializedMint: DeserializedCompressedMint = {
        base: {
            mintAuthorityOption: 1,
            mintAuthority: new Uint8Array(32).fill(0x11),
            supply: 1000000n,
            decimals: 9,
            isInitialized: true,
            freezeAuthorityOption: 0,
            freezeAuthority: new Uint8Array(32),
        },
        mintContext: {
            version: 0,
            cmintDecompressed: false,
            splMint: new Uint8Array(32).fill(0x22),
            mintSigner: new Uint8Array(32).fill(0x33),
            bump: 254,
        },
        metadataExtensionIndex: 0,
    };

    const mockAccount: CompressedAccount = {
        hash: new Uint8Array(32).fill(0xaa),
        address: new Uint8Array(32).fill(0xbb),
        owner: MOCK_CTOKEN_PROGRAM,
        lamports: 0n,
        data: {
            discriminator: new Uint8Array(8),
            data: mintData,
            dataHash: new Uint8Array(32),
        },
        leafIndex: 42,
        treeInfo: createMockTreeInfo(TreeType.StateV2),
        proveByIndex: true,
        seq: 5n,
        slotCreated: 100n,
    };

    return {
        account: mockAccount,
        mint: mockDeserializedMint,
        mintSigner: MOCK_MINT_SIGNER,
        leafIndex: 42,
        rootIndex: 10,
        proveByIndex: true,
        merkleTree: MOCK_TREE,
        outOutputQueue: MOCK_QUEUE,
        proof: null,
        metadataExtensionIndex: 0,
        ...overrides,
    };
}

// ============================================================================
// MOCK SPL INTERFACE INFO
// ============================================================================

/**
 * Creates a mock SplInterfaceInfo with consistent test values.
 */
export function createMockSplInterfaceInfo(): SplInterfaceInfo {
    return {
        poolAddress: MOCK_POOL,
        tokenProgram: SPL_TOKEN_PROGRAM_ID,
        poolIndex: 0,
        bump: 255,
        isInitialized: true,
    };
}

// ============================================================================
// PROOF HELPERS
// ============================================================================

/**
 * Creates a mock proof input for validity proof responses.
 */
export function createProofInput(hashByte: number, rootIndex: number) {
    return {
        hash: new Uint8Array(32).fill(hashByte),
        root: new Uint8Array(32),
        rootIndex: { rootIndex, proveByIndex: false },
        leafIndex: 0,
        treeInfo: createMockTreeInfo(TreeType.StateV2),
    };
}

/**
 * Creates a mock validity proof response.
 */
export function createMockProof(
    accountInputs: Array<{ hashByte: number; rootIndex: number }> = [],
) {
    return {
        proof: {
            a: new Uint8Array(32),
            b: new Uint8Array(64),
            c: new Uint8Array(32),
        },
        accounts: accountInputs.map((a) =>
            createProofInput(a.hashByte, a.rootIndex),
        ),
        addresses: [],
    };
}

/**
 * Creates a mock token account with a specific hash byte and leaf index.
 */
export function createMockAccountWithHash(
    amount: bigint,
    hashByte: number,
    leafIndex: number,
    delegate: ReturnType<typeof address> | null = null,
): CompressedTokenAccount {
    const account = createMockTokenAccount(amount);
    account.account.hash = new Uint8Array(32).fill(hashByte);
    account.account.leafIndex = leafIndex;
    account.token.delegate = delegate;
    return account;
}

/**
 * Creates a mock indexer that returns accounts and proof for transfer builders.
 */
export function createTransferMockIndexer(
    accounts: CompressedTokenAccount[],
    proofInputs: Array<{ hashByte: number; rootIndex: number }>,
): LightIndexer {
    return createMockIndexer({
        getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
            context: { slot: 100n },
            value: { items: accounts, cursor: null },
        }),
        getValidityProof: vi.fn().mockResolvedValue({
            context: { slot: 100n },
            value: createMockProof(proofInputs),
        }),
    });
}
