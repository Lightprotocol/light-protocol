/**
 * Unit tests for load functions and actions.
 *
 * Tests for:
 * - loadTokenAccountsForTransfer
 * - loadAllTokenAccounts
 * - loadTokenAccount
 * - loadCompressedAccount
 * - loadCompressedAccountByHash
 * - getValidityProofForAccounts
 * - getOutputTreeInfo
 * - needsValidityProof
 * - buildCompressedTransfer
 * - loadMintContext
 * - getMintDecimals
 */

import { describe, it, expect, vi } from 'vitest';
import { address } from '@solana/addresses';

import {
    loadTokenAccountsForTransfer,
    loadAllTokenAccounts,
    loadTokenAccount,
    loadCompressedAccount,
    loadCompressedAccountByHash,
    getValidityProofForAccounts,
    getOutputTreeInfo,
    needsValidityProof,
    buildCompressedTransfer,
    loadMintContext,
    getMintDecimals,
    getMintInterface,
    getAtaInterface,
    deserializeCompressedMint,
    IndexerError,
    IndexerErrorCode,
    TreeType,
    DISCRIMINATOR,
    EXTENSION_DISCRIMINANT,
    type LightIndexer,
    type CompressedAccount,
    type CompressedTokenAccount,
} from '../../src/index.js';
import {
    createMockTokenAccount,
    createMockTreeInfo,
    createMockIndexer,
    createMockCompressedMintData,
    createBase64MintData,
    createMockAccountWithHash,
    createProofInput,
    MOCK_OWNER,
    MOCK_MINT,
    MOCK_CTOKEN_PROGRAM,
} from './helpers.js';

// ============================================================================
// TESTS: loadTokenAccountsForTransfer
// ============================================================================

describe('loadTokenAccountsForTransfer', () => {
    it('returns inputs, proof, and totalAmount on success', async () => {
        const accounts = [
            createMockTokenAccount(500n),
            createMockTokenAccount(300n),
        ];

        const mockProof = {
            proof: { a: new Uint8Array(32), b: new Uint8Array(64), c: new Uint8Array(32) },
            accounts: [],
            addresses: [],
        };

        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: accounts, cursor: null },
            }),
            getValidityProof: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: mockProof,
            }),
        });

        const result = await loadTokenAccountsForTransfer(
            indexer,
            MOCK_OWNER,
            600n,
        );

        expect(result.inputs).toHaveLength(2);
        expect(result.proof).toBe(mockProof);
        expect(result.totalAmount).toBe(800n);

        for (const input of result.inputs) {
            expect(input.merkleContext).toBeDefined();
            expect(input.merkleContext.tree).toBeDefined();
            expect(input.merkleContext.queue).toBeDefined();
        }
    });

    it('throws IndexerError with NotFound when no accounts exist', async () => {
        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: [], cursor: null },
            }),
        });

        await expect(
            loadTokenAccountsForTransfer(indexer, MOCK_OWNER, 100n),
        ).rejects.toThrow(IndexerError);

        try {
            await loadTokenAccountsForTransfer(indexer, MOCK_OWNER, 100n);
        } catch (e) {
            expect(e).toBeInstanceOf(IndexerError);
            expect((e as IndexerError).code).toBe(IndexerErrorCode.NotFound);
        }
    });

    it('respects maxInputs option during selection', async () => {
        const accounts = [
            createMockTokenAccount(500n),
            createMockTokenAccount(400n),
            createMockTokenAccount(300n),
        ];

        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: accounts, cursor: null },
            }),
        });

        await expect(
            loadTokenAccountsForTransfer(indexer, MOCK_OWNER, 700n, {
                maxInputs: 1,
            }),
        ).rejects.toMatchObject({
            code: IndexerErrorCode.InsufficientBalance,
        });
    });

    it('throws IndexerError with InsufficientBalance when balance is too low', async () => {
        const accounts = [createMockTokenAccount(50n)];

        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: accounts, cursor: null },
            }),
        });

        await expect(
            loadTokenAccountsForTransfer(indexer, MOCK_OWNER, 1000n),
        ).rejects.toThrow(IndexerError);

        try {
            await loadTokenAccountsForTransfer(indexer, MOCK_OWNER, 1000n);
        } catch (e) {
            expect(e).toBeInstanceOf(IndexerError);
            expect((e as IndexerError).code).toBe(
                IndexerErrorCode.InsufficientBalance,
            );
        }
    });
});

// ============================================================================
// TESTS: loadAllTokenAccounts
// ============================================================================

describe('loadAllTokenAccounts', () => {
    it('returns items from a single page with no cursor', async () => {
        const accounts = [
            createMockTokenAccount(100n),
            createMockTokenAccount(200n),
        ];

        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: accounts, cursor: null },
            }),
        });

        const result = await loadAllTokenAccounts(indexer, MOCK_OWNER);

        expect(result).toHaveLength(2);
        expect(result[0].token.amount).toBe(100n);
        expect(result[1].token.amount).toBe(200n);
    });

    it('paginates through multiple pages using cursor', async () => {
        const page1 = [createMockTokenAccount(100n)];
        const page2 = [createMockTokenAccount(200n)];

        const mockFn = vi
            .fn()
            .mockResolvedValueOnce({
                context: { slot: 100n },
                value: { items: page1, cursor: 'cursor-abc' },
            })
            .mockResolvedValueOnce({
                context: { slot: 101n },
                value: { items: page2, cursor: null },
            });

        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: mockFn,
        });

        const result = await loadAllTokenAccounts(indexer, MOCK_OWNER);

        expect(result).toHaveLength(2);
        expect(result[0].token.amount).toBe(100n);
        expect(result[1].token.amount).toBe(200n);
        expect(mockFn).toHaveBeenCalledTimes(2);
    });

    it('throws after exceeding maximum page limit', async () => {
        const mockFn = vi.fn().mockResolvedValue({
            context: { slot: 100n },
            value: { items: [createMockTokenAccount(1n)], cursor: 'next' },
        });

        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: mockFn,
        });

        await expect(
            loadAllTokenAccounts(indexer, MOCK_OWNER),
        ).rejects.toThrow('Pagination exceeded maximum of 100 pages');
    });
});

// ============================================================================
// TESTS: loadTokenAccount
// ============================================================================

describe('loadTokenAccount', () => {
    it('returns the first matching account', async () => {
        const account = createMockTokenAccount(500n);

        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: [account], cursor: null },
            }),
        });

        const result = await loadTokenAccount(indexer, MOCK_OWNER, MOCK_MINT);

        expect(result).not.toBeNull();
        expect(result!.token.amount).toBe(500n);
    });

    it('returns null when no accounts match', async () => {
        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: [], cursor: null },
            }),
        });

        const result = await loadTokenAccount(indexer, MOCK_OWNER, MOCK_MINT);

        expect(result).toBeNull();
    });
});

// ============================================================================
// TESTS: loadCompressedAccount
// ============================================================================

describe('loadCompressedAccount', () => {
    it('returns account when found', async () => {
        const mockAccount: CompressedAccount = {
            hash: new Uint8Array(32).fill(0xab),
            address: null,
            owner: address('cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m'),
            lamports: 1000n,
            data: null,
            leafIndex: 5,
            treeInfo: createMockTreeInfo(TreeType.StateV2),
            proveByIndex: false,
            seq: 10n,
            slotCreated: 42n,
        };

        const indexer = createMockIndexer({
            getCompressedAccount: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: mockAccount,
            }),
        });

        const result = await loadCompressedAccount(indexer, new Uint8Array(32));
        expect(result).not.toBeNull();
        expect(result!.lamports).toBe(1000n);
        expect(result!.leafIndex).toBe(5);
    });

    it('returns null when not found', async () => {
        const indexer = createMockIndexer({
            getCompressedAccount: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: null,
            }),
        });

        const result = await loadCompressedAccount(indexer, new Uint8Array(32));
        expect(result).toBeNull();
    });
});

// ============================================================================
// TESTS: loadCompressedAccountByHash
// ============================================================================

describe('loadCompressedAccountByHash', () => {
    it('returns account when found', async () => {
        const mockAccount: CompressedAccount = {
            hash: new Uint8Array(32).fill(0xcd),
            address: null,
            owner: address('cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m'),
            lamports: 2000n,
            data: null,
            leafIndex: 10,
            treeInfo: createMockTreeInfo(TreeType.StateV2),
            proveByIndex: true,
            seq: 20n,
            slotCreated: 100n,
        };

        const indexer = createMockIndexer({
            getCompressedAccountByHash: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: mockAccount,
            }),
        });

        const result = await loadCompressedAccountByHash(indexer, new Uint8Array(32));
        expect(result).not.toBeNull();
        expect(result!.lamports).toBe(2000n);
        expect(result!.proveByIndex).toBe(true);
    });

    it('returns null when not found', async () => {
        const indexer = createMockIndexer({
            getCompressedAccountByHash: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: null,
            }),
        });

        const result = await loadCompressedAccountByHash(indexer, new Uint8Array(32));
        expect(result).toBeNull();
    });
});

// ============================================================================
// TESTS: getValidityProofForAccounts
// ============================================================================

describe('getValidityProofForAccounts', () => {
    it('fetches proof using account hashes', async () => {
        const account1 = createMockTokenAccount(100n);
        account1.account.hash = new Uint8Array(32).fill(0x11);
        const account2 = createMockTokenAccount(200n);
        account2.account.hash = new Uint8Array(32).fill(0x22);

        const mockProof = {
            proof: { a: new Uint8Array(32), b: new Uint8Array(64), c: new Uint8Array(32) },
            accounts: [],
            addresses: [],
        };

        const getValidityProofFn = vi.fn().mockResolvedValue({
            context: { slot: 100n },
            value: mockProof,
        });

        const indexer = createMockIndexer({
            getValidityProof: getValidityProofFn,
        });

        const result = await getValidityProofForAccounts(indexer, [account1, account2]);

        expect(result).toBe(mockProof);
        expect(getValidityProofFn).toHaveBeenCalledTimes(1);
        const calledHashes = getValidityProofFn.mock.calls[0][0];
        expect(calledHashes).toHaveLength(2);
        expect(calledHashes[0]).toEqual(new Uint8Array(32).fill(0x11));
        expect(calledHashes[1]).toEqual(new Uint8Array(32).fill(0x22));
    });

    it('handles empty accounts array', async () => {
        const mockProof = {
            proof: { a: new Uint8Array(32), b: new Uint8Array(64), c: new Uint8Array(32) },
            accounts: [],
            addresses: [],
        };

        const indexer = createMockIndexer({
            getValidityProof: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: mockProof,
            }),
        });

        const result = await getValidityProofForAccounts(indexer, []);
        expect(result).toBe(mockProof);
    });
});

// ============================================================================
// TESTS: getOutputTreeInfo
// ============================================================================

describe('getOutputTreeInfo', () => {
    it('returns nextTreeInfo when present', () => {
        const nextTree = createMockTreeInfo(TreeType.StateV2);
        nextTree.tree = address('GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy');

        const currentTree = createMockTreeInfo(TreeType.StateV2, nextTree);

        const result = getOutputTreeInfo(currentTree);

        expect(result).toBe(nextTree);
        expect(result.tree).toBe(nextTree.tree);
    });

    it('returns the current tree when no next tree exists', () => {
        const currentTree = createMockTreeInfo(TreeType.StateV2);

        const result = getOutputTreeInfo(currentTree);

        expect(result).toBe(currentTree);
    });
});

// ============================================================================
// TESTS: needsValidityProof
// ============================================================================

describe('needsValidityProof', () => {
    it('returns true when proveByIndex is false', () => {
        const account: CompressedAccount = {
            hash: new Uint8Array(32),
            address: null,
            owner: address('cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m'),
            lamports: 0n,
            data: null,
            leafIndex: 0,
            treeInfo: createMockTreeInfo(TreeType.StateV2),
            proveByIndex: false,
            seq: null,
            slotCreated: 0n,
        };

        expect(needsValidityProof(account)).toBe(true);
    });

    it('returns false when proveByIndex is true', () => {
        const account: CompressedAccount = {
            hash: new Uint8Array(32),
            address: null,
            owner: address('cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m'),
            lamports: 0n,
            data: null,
            leafIndex: 0,
            treeInfo: createMockTreeInfo(TreeType.StateV2),
            proveByIndex: true,
            seq: null,
            slotCreated: 0n,
        };

        expect(needsValidityProof(account)).toBe(false);
    });
});

// ============================================================================
// TESTS: buildCompressedTransfer
// ============================================================================

describe('buildCompressedTransfer', () => {
    const RECIPIENT = address('GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy');
    const FEE_PAYER = address('BPFLoaderUpgradeab1e11111111111111111111111');
    const DELEGATE = address('Sysvar1111111111111111111111111111111111111');
    const ALT_TREE = address('Vote111111111111111111111111111111111111111');
    const ALT_QUEUE = address('11111111111111111111111111111111');

    function decodeTransfer2OutputQueueIndex(data: Uint8Array): number {
        return data[5];
    }

    function decodeTransfer2MaxTopUp(data: Uint8Array): number {
        const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
        return view.getUint16(6, true);
    }

    it('builds Transfer2 instruction with correct discriminator', async () => {
        const accounts = [createMockAccountWithHash(1000n, 0xab, 5)];
        const mockProof = {
            proof: { a: new Uint8Array(32), b: new Uint8Array(64), c: new Uint8Array(32) },
            accounts: [createProofInput(0xab, 10)],
            addresses: [],
        };
        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: accounts, cursor: null },
            }),
            getValidityProof: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: mockProof,
            }),
        });

        const result = await buildCompressedTransfer({
            indexer,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            amount: 500n,
            recipientOwner: RECIPIENT,
            feePayer: FEE_PAYER,
        });

        expect(result.instruction.data[0]).toBe(DISCRIMINATOR.TRANSFER2);
        expect(result.totalInputAmount).toBe(1000n);
    });

    it('uses Rust-compatible default maxTopUp (u16::MAX)', async () => {
        const accounts = [createMockAccountWithHash(1000n, 0xab, 5)];
        const mockProof = {
            proof: { a: new Uint8Array(32), b: new Uint8Array(64), c: new Uint8Array(32) },
            accounts: [createProofInput(0xab, 10)],
            addresses: [],
        };
        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: accounts, cursor: null },
            }),
            getValidityProof: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: mockProof,
            }),
        });

        const result = await buildCompressedTransfer({
            indexer,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            amount: 500n,
            recipientOwner: RECIPIENT,
            feePayer: FEE_PAYER,
        });

        expect(decodeTransfer2MaxTopUp(result.instruction.data)).toBe(65535);
    });

    it('uses explicit maxTopUp when provided', async () => {
        const accounts = [createMockAccountWithHash(1000n, 0xab, 5)];
        const mockProof = {
            proof: { a: new Uint8Array(32), b: new Uint8Array(64), c: new Uint8Array(32) },
            accounts: [createProofInput(0xab, 10)],
            addresses: [],
        };
        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: accounts, cursor: null },
            }),
            getValidityProof: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: mockProof,
            }),
        });

        const result = await buildCompressedTransfer({
            indexer,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            amount: 500n,
            recipientOwner: RECIPIENT,
            feePayer: FEE_PAYER,
            maxTopUp: 321,
        });

        expect(decodeTransfer2MaxTopUp(result.instruction.data)).toBe(321);
    });

    it('uses nextTreeInfo queue for output queue when present', async () => {
        const account = createMockAccountWithHash(1000n, 0xab, 5);
        account.account.treeInfo = createMockTreeInfo(TreeType.StateV2, {
            tree: ALT_TREE,
            queue: ALT_QUEUE,
            treeType: TreeType.StateV2,
        });

        const mockProof = {
            proof: { a: new Uint8Array(32), b: new Uint8Array(64), c: new Uint8Array(32) },
            accounts: [createProofInput(0xab, 10)],
            addresses: [],
        };
        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: [account], cursor: null },
            }),
            getValidityProof: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: mockProof,
            }),
        });

        const result = await buildCompressedTransfer({
            indexer,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            amount: 500n,
            recipientOwner: RECIPIENT,
            feePayer: FEE_PAYER,
        });

        const outputQueueIdx = decodeTransfer2OutputQueueIndex(
            result.instruction.data,
        );
        const packedAccountsOffset = 7;
        expect(
            result.instruction.accounts[packedAccountsOffset + outputQueueIdx]
                .address,
        ).toBe(ALT_QUEUE);
    });

    it('returns correct inputs, proof, and totalInputAmount', async () => {
        const accounts = [
            createMockAccountWithHash(600n, 0x11, 1),
            createMockAccountWithHash(400n, 0x22, 2),
        ];
        const mockProof = {
            proof: { a: new Uint8Array(32), b: new Uint8Array(64), c: new Uint8Array(32) },
            accounts: [createProofInput(0x22, 6), createProofInput(0x11, 5)],
            addresses: [],
        };
        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: accounts, cursor: null },
            }),
            getValidityProof: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: mockProof,
            }),
        });

        const result = await buildCompressedTransfer({
            indexer,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            amount: 800n,
            recipientOwner: RECIPIENT,
            feePayer: FEE_PAYER,
        });

        expect(result.inputs).toHaveLength(2);
        expect(result.proof).toBe(mockProof);
        expect(result.totalInputAmount).toBe(1000n);
    });

    it('forwards maxInputs to selection via loadTokenAccountsForTransfer', async () => {
        const accounts = [
            createMockAccountWithHash(500n, 0x11, 1),
            createMockAccountWithHash(400n, 0x22, 2),
            createMockAccountWithHash(300n, 0x33, 3),
        ];
        const mockProof = {
            proof: { a: new Uint8Array(32), b: new Uint8Array(64), c: new Uint8Array(32) },
            accounts: [createProofInput(0x11, 7)],
            addresses: [],
        };
        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: accounts, cursor: null },
            }),
            getValidityProof: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: mockProof,
            }),
        });

        await expect(
            buildCompressedTransfer({
                indexer,
                owner: MOCK_OWNER,
                mint: MOCK_MINT,
                amount: 700n,
                recipientOwner: RECIPIENT,
                feePayer: FEE_PAYER,
                maxInputs: 1,
            }),
        ).rejects.toMatchObject({
            code: IndexerErrorCode.InsufficientBalance,
        });
    });

    it('includes delegate account in packed accounts when selected input has delegate', async () => {
        const accounts = [
            createMockAccountWithHash(1000n, 0xab, 5, DELEGATE),
        ];
        const mockProof = {
            proof: { a: new Uint8Array(32), b: new Uint8Array(64), c: new Uint8Array(32) },
            accounts: [createProofInput(0xab, 10)],
            addresses: [],
        };
        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: accounts, cursor: null },
            }),
            getValidityProof: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: mockProof,
            }),
        });

        const result = await buildCompressedTransfer({
            indexer,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            amount: 300n,
            recipientOwner: RECIPIENT,
            feePayer: FEE_PAYER,
        });
        expect(
            result.instruction.accounts.some((acc) => acc.address === DELEGATE),
        ).toBe(true);
    });

    it('throws InvalidResponse when proof does not contain selected input hash', async () => {
        const accounts = [createMockAccountWithHash(1000n, 0xab, 5)];
        const mockProof = {
            proof: { a: new Uint8Array(32), b: new Uint8Array(64), c: new Uint8Array(32) },
            accounts: [createProofInput(0xcd, 99)],
            addresses: [],
        };
        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: accounts, cursor: null },
            }),
            getValidityProof: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: mockProof,
            }),
        });

        await expect(
            buildCompressedTransfer({
                indexer,
                owner: MOCK_OWNER,
                mint: MOCK_MINT,
                amount: 100n,
                recipientOwner: RECIPIENT,
                feePayer: FEE_PAYER,
            }),
        ).rejects.toMatchObject({
            code: IndexerErrorCode.InvalidResponse,
        });
    });

    it('throws when insufficient balance', async () => {
        const accounts = [createMockAccountWithHash(100n, 0xab, 5)];

        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: accounts, cursor: null },
            }),
        });

        await expect(
            buildCompressedTransfer({
                indexer,
                owner: MOCK_OWNER,
                mint: MOCK_MINT,
                amount: 1000n,
                recipientOwner: RECIPIENT,
                feePayer: FEE_PAYER,
            }),
        ).rejects.toThrow(IndexerError);
    });
});

// ============================================================================
// TESTS: loadMintContext
// ============================================================================

describe('loadMintContext', () => {
    const MINT_SIGNER = address('GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy');

    function createMockMintData(): Uint8Array {
        // Create a minimal 149-byte compressed mint data buffer
        const data = new Uint8Array(149);
        const view = new DataView(data.buffer);
        // mintAuthorityOption = 1 (has authority)
        view.setUint32(0, 1, true);
        // supply
        view.setBigUint64(36, 1000000n, true);
        // decimals
        data[44] = 9;
        // isInitialized
        data[45] = 1;
        // MintContext at offset 82
        data[82] = 0; // version
        data[83] = 0; // cmintDecompressed
        data[148] = 255; // bump
        return data;
    }

    it('loads and deserializes a compressed mint', async () => {
        const mintData = createMockMintData();
        const mockAccount: CompressedAccount = {
            hash: new Uint8Array(32).fill(0xaa),
            address: new Uint8Array(32).fill(0xbb),
            owner: address('cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m'),
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

        const indexer = createMockIndexer({
            getCompressedAccount: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: mockAccount,
            }),
        });

        const ctx = await loadMintContext(indexer, MINT_SIGNER);

        expect(ctx.leafIndex).toBe(42);
        expect(ctx.proveByIndex).toBe(true);
        expect(ctx.mint.base.decimals).toBe(9);
        expect(ctx.mint.base.supply).toBe(1000000n);
        expect(ctx.mintSigner).toBe(MINT_SIGNER);
        // prove-by-index means no proof fetch
        expect(ctx.proof).toBeNull();
    });

    it('throws when mint not found', async () => {
        const indexer = createMockIndexer({
            getCompressedAccount: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: null,
            }),
        });

        await expect(
            loadMintContext(indexer, MINT_SIGNER),
        ).rejects.toThrow(IndexerError);
    });
});

// ============================================================================
// TESTS: getMintDecimals
// ============================================================================

describe('getMintDecimals', () => {
    it('returns decimals from on-chain mint', async () => {
        // Create a minimal SPL mint buffer (82 bytes)
        const mintBytes = new Uint8Array(82);
        mintBytes[44] = 6; // decimals = 6

        const base64Data = btoa(String.fromCharCode(...mintBytes));

        const mockRpc = {
            getAccountInfo: vi.fn().mockResolvedValue({
                value: {
                    owner: address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
                    data: [base64Data, 'base64'],
                },
            }),
        };

        const result = await getMintDecimals(mockRpc, MOCK_MINT);
        expect(result).toBe(6);
    });

    it('throws when mint not found', async () => {
        const mockRpc = {
            getAccountInfo: vi.fn().mockResolvedValue({ value: null }),
        };

        await expect(
            getMintDecimals(mockRpc, MOCK_MINT),
        ).rejects.toThrow('Mint account not found');
    });
});

// ============================================================================
// TESTS: deserializeCompressedMint
// ============================================================================

describe('deserializeCompressedMint', () => {
    it('parses valid 149-byte buffer', () => {
        const data = createMockCompressedMintData(6, 500000n);
        const result = deserializeCompressedMint(data);

        expect(result.base.decimals).toBe(6);
        expect(result.base.supply).toBe(500000n);
        expect(result.base.mintAuthorityOption).toBe(1);
        expect(result.base.isInitialized).toBe(true);
    });

    it('parses mintContext fields', () => {
        const data = createMockCompressedMintData();
        const result = deserializeCompressedMint(data);

        expect(result.mintContext.version).toBe(0);
        expect(result.mintContext.cmintDecompressed).toBe(false);
        expect(result.mintContext.bump).toBe(254);
        expect(result.mintContext.splMint).toEqual(new Uint8Array(32).fill(0x22));
        expect(result.mintContext.mintSigner).toEqual(new Uint8Array(32).fill(0x33));
    });

    it('throws on data < 149 bytes', () => {
        const shortData = new Uint8Array(100);
        expect(() => deserializeCompressedMint(shortData)).toThrow(
            'Compressed mint data too short',
        );
    });

    it('returns metadataExtensionIndex = -1 when no extensions', () => {
        const data = createMockCompressedMintData();
        const result = deserializeCompressedMint(data);

        expect(result.metadataExtensionIndex).toBe(-1);
    });

    it('finds TOKEN_METADATA extension when present', () => {
        // Create data with extensions: 4-byte vec len + 2-byte discriminant
        const base = createMockCompressedMintData();
        const extData = new Uint8Array(base.length + 6);
        extData.set(base);

        const extView = new DataView(extData.buffer);
        // Vec length = 1 extension
        extView.setUint32(149, 1, true);
        // TOKEN_METADATA discriminant = 19
        extView.setUint16(153, EXTENSION_DISCRIMINANT.TOKEN_METADATA, true);

        const result = deserializeCompressedMint(extData);
        expect(result.metadataExtensionIndex).toBe(0);
    });
});

// ============================================================================
// TESTS: getMintInterface
// ============================================================================

describe('getMintInterface', () => {
    it('returns exists=false when RPC returns null', async () => {
        const mockRpc = {
            getAccountInfo: vi.fn().mockResolvedValue({ value: null }),
        };

        const result = await getMintInterface(mockRpc, MOCK_MINT);

        expect(result.exists).toBe(false);
        expect(result.decimals).toBe(0);
        expect(result.supply).toBe(0n);
        expect(result.hasFreezeAuthority).toBe(false);
    });

    it('parses decimals, supply, freezeAuthority from valid mint data', async () => {
        const mintData = createBase64MintData(9, 5000000n, true);
        const mockRpc = {
            getAccountInfo: vi.fn().mockResolvedValue({
                value: {
                    owner: address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
                    data: mintData,
                },
            }),
        };

        const result = await getMintInterface(mockRpc, MOCK_MINT);

        expect(result.exists).toBe(true);
        expect(result.decimals).toBe(9);
        expect(result.supply).toBe(5000000n);
        expect(result.hasFreezeAuthority).toBe(true);
    });

    it('handles data < 82 bytes gracefully', async () => {
        const shortData = btoa(String.fromCharCode(...new Uint8Array(40)));
        const mockRpc = {
            getAccountInfo: vi.fn().mockResolvedValue({
                value: {
                    owner: address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
                    data: [shortData, 'base64'],
                },
            }),
        };

        const result = await getMintInterface(mockRpc, MOCK_MINT);

        expect(result.exists).toBe(true);
        expect(result.decimals).toBe(0);
        expect(result.supply).toBe(0n);
    });
});

// ============================================================================
// TESTS: getAtaInterface
// ============================================================================

describe('getAtaInterface', () => {
    it('aggregates hot + cold + spl balances', async () => {
        // Build a mock 72-byte account with balance=500 at offset 64
        const accountBytes = new Uint8Array(72);
        const view = new DataView(accountBytes.buffer);
        view.setBigUint64(64, 500n, true);
        const base64 = btoa(String.fromCharCode(...accountBytes));

        const hotAddr = address('Vote111111111111111111111111111111111111111');
        const splAddr = address('11111111111111111111111111111111');

        const mockRpc = {
            getAccountInfo: vi.fn().mockResolvedValue({
                value: {
                    owner: MOCK_CTOKEN_PROGRAM,
                    data: [base64, 'base64'],
                },
            }),
        };

        const coldAccount = createMockTokenAccount(300n);
        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: [coldAccount], cursor: null },
            }),
        });

        const result = await getAtaInterface(
            mockRpc,
            indexer,
            MOCK_OWNER,
            MOCK_MINT,
            hotAddr,
            splAddr,
        );

        expect(result.hotBalance).toBe(500n);
        expect(result.splBalance).toBe(500n);
        expect(result.coldBalance).toBe(300n);
        expect(result.totalBalance).toBe(1300n);
        expect(result.coldAccountCount).toBe(1);
    });

    it('returns zeros when no accounts found and no hot/spl provided', async () => {
        const mockRpc = {
            getAccountInfo: vi.fn().mockResolvedValue({ value: null }),
        };

        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: [], cursor: null },
            }),
        });

        const result = await getAtaInterface(
            mockRpc,
            indexer,
            MOCK_OWNER,
            MOCK_MINT,
        );

        expect(result.hotBalance).toBe(0n);
        expect(result.coldBalance).toBe(0n);
        expect(result.splBalance).toBe(0n);
        expect(result.totalBalance).toBe(0n);
        expect(result.sources).toHaveLength(0);
    });
});
