/**
 * Unit tests for PhotonIndexer and isLightIndexerAvailable.
 *
 * Tests error handling paths in the RPC client by mocking globalThis.fetch.
 */

import { describe, it, expect, vi, afterEach } from 'vitest';

import { PhotonIndexer, isLightIndexerAvailable } from '../../src/index.js';

import { IndexerError, IndexerErrorCode, TreeType } from '@lightprotocol/token-sdk';

// ============================================================================
// SETUP
// ============================================================================

const ENDPOINT = 'https://test.photon.endpoint';
const originalFetch = globalThis.fetch;

afterEach(() => {
    globalThis.fetch = originalFetch;
});

/**
 * Helper to create a mock Response that provides both .text() and .json().
 * The indexer now uses response.text() for big-number-safe parsing.
 */
function mockResponse(body: unknown, ok = true, status = 200, statusText = 'OK') {
    const text = JSON.stringify(body);
    return {
        ok,
        status,
        statusText,
        text: vi.fn().mockResolvedValue(text),
        json: vi.fn().mockResolvedValue(body),
    };
}

// ============================================================================
// TESTS: PhotonIndexer error handling
// ============================================================================

describe('PhotonIndexer', () => {
    it('throws IndexerError with NetworkError on network failure', async () => {
        globalThis.fetch = vi.fn().mockRejectedValue(new Error('ECONNREFUSED'));

        const indexer = new PhotonIndexer(ENDPOINT);

        await expect(
            indexer.getCompressedAccount(new Uint8Array(32)),
        ).rejects.toThrow(IndexerError);

        try {
            await indexer.getCompressedAccount(new Uint8Array(32));
        } catch (e) {
            expect(e).toBeInstanceOf(IndexerError);
            expect((e as IndexerError).code).toBe(IndexerErrorCode.NetworkError);
        }
    });

    it('throws IndexerError with NetworkError on HTTP error status', async () => {
        globalThis.fetch = vi.fn().mockResolvedValue({
            ok: false,
            status: 500,
            statusText: 'Internal Server Error',
        });

        const indexer = new PhotonIndexer(ENDPOINT);

        await expect(
            indexer.getCompressedAccount(new Uint8Array(32)),
        ).rejects.toThrow(IndexerError);

        try {
            await indexer.getCompressedAccount(new Uint8Array(32));
        } catch (e) {
            expect(e).toBeInstanceOf(IndexerError);
            expect((e as IndexerError).code).toBe(IndexerErrorCode.NetworkError);
            expect((e as IndexerError).message).toContain('500');
        }
    });

    it('throws IndexerError with InvalidResponse on invalid JSON', async () => {
        globalThis.fetch = vi.fn().mockResolvedValue({
            ok: true,
            text: vi.fn().mockResolvedValue('not valid json {{{'),
        });

        const indexer = new PhotonIndexer(ENDPOINT);

        await expect(
            indexer.getCompressedAccount(new Uint8Array(32)),
        ).rejects.toThrow(IndexerError);

        try {
            await indexer.getCompressedAccount(new Uint8Array(32));
        } catch (e) {
            expect(e).toBeInstanceOf(IndexerError);
            expect((e as IndexerError).code).toBe(
                IndexerErrorCode.InvalidResponse,
            );
        }
    });

    it('throws IndexerError with RpcError on JSON-RPC error response', async () => {
        globalThis.fetch = vi.fn().mockResolvedValue(
            mockResponse({
                jsonrpc: '2.0',
                id: '1',
                error: { code: -32600, message: 'Invalid' },
            }),
        );

        const indexer = new PhotonIndexer(ENDPOINT);

        await expect(
            indexer.getCompressedAccount(new Uint8Array(32)),
        ).rejects.toThrow(IndexerError);

        try {
            await indexer.getCompressedAccount(new Uint8Array(32));
        } catch (e) {
            expect(e).toBeInstanceOf(IndexerError);
            expect((e as IndexerError).code).toBe(IndexerErrorCode.RpcError);
            expect((e as IndexerError).message).toContain('-32600');
        }
    });

    it('throws IndexerError with InvalidResponse when result is missing', async () => {
        globalThis.fetch = vi.fn().mockResolvedValue(
            mockResponse({
                jsonrpc: '2.0',
                id: '1',
            }),
        );

        const indexer = new PhotonIndexer(ENDPOINT);

        await expect(
            indexer.getCompressedAccount(new Uint8Array(32)),
        ).rejects.toThrow(IndexerError);

        try {
            await indexer.getCompressedAccount(new Uint8Array(32));
        } catch (e) {
            expect(e).toBeInstanceOf(IndexerError);
            expect((e as IndexerError).code).toBe(
                IndexerErrorCode.InvalidResponse,
            );
            expect((e as IndexerError).message).toContain('Missing result');
        }
    });

    it('throws IndexerError for V1 tree type in account response', async () => {
        globalThis.fetch = vi.fn().mockResolvedValue(
            mockResponse({
                jsonrpc: '2.0',
                id: '1',
                result: {
                    context: { slot: 100 },
                    value: {
                        hash: '11111111111111111111111111111111',
                        address: null,
                        data: null,
                        lamports: '0',
                        owner: 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
                        leafIndex: 0,
                        seq: null,
                        slotCreated: '0',
                        merkleContext: {
                            tree: 'amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx',
                            queue: 'SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7',
                            treeType: TreeType.StateV1,
                        },
                        proveByIndex: false,
                    },
                },
            }),
        );

        const indexer = new PhotonIndexer(ENDPOINT);

        await expect(
            indexer.getCompressedAccount(new Uint8Array(32)),
        ).rejects.toThrow(IndexerError);

        try {
            await indexer.getCompressedAccount(new Uint8Array(32));
        } catch (e) {
            expect(e).toBeInstanceOf(IndexerError);
            expect((e as IndexerError).code).toBe(
                IndexerErrorCode.InvalidResponse,
            );
            expect((e as IndexerError).message).toContain(
                'V1 tree types are not supported',
            );
        }
    });

    it('successfully parses a valid compressed account response', async () => {
        globalThis.fetch = vi.fn().mockResolvedValue(
            mockResponse({
                jsonrpc: '2.0',
                id: '1',
                result: {
                    context: { slot: 42 },
                    value: {
                        hash: '11111111111111111111111111111111',
                        address: 'So11111111111111111111111111111111111111112',
                        data: null,
                        lamports: '1000000',
                        owner: 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
                        leafIndex: 7,
                        seq: 99,
                        slotCreated: '123',
                        merkleContext: {
                            tree: 'amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx',
                            queue: 'SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7',
                            treeType: TreeType.StateV2,
                        },
                        proveByIndex: true,
                    },
                },
            }),
        );

        const indexer = new PhotonIndexer(ENDPOINT);
        const result = await indexer.getCompressedAccount(new Uint8Array(32));

        expect(result.context.slot).toBe(42n);
        expect(result.value).not.toBeNull();
        expect(result.value!.lamports).toBe(1000000n);
        expect(result.value!.leafIndex).toBe(7);
        expect(result.value!.seq).toBe(99n);
        expect(result.value!.slotCreated).toBe(123n);
        expect(result.value!.proveByIndex).toBe(true);
        expect(result.value!.address).not.toBeNull();
        expect(result.value!.data).toBeNull();
    });

    it('successfully parses a null account response', async () => {
        globalThis.fetch = vi.fn().mockResolvedValue(
            mockResponse({
                jsonrpc: '2.0',
                id: '1',
                result: {
                    context: { slot: 10 },
                    value: null,
                },
            }),
        );

        const indexer = new PhotonIndexer(ENDPOINT);
        const result = await indexer.getCompressedAccount(new Uint8Array(32));
        expect(result.value).toBeNull();
    });

    it('successfully parses token accounts response', async () => {
        globalThis.fetch = vi.fn().mockResolvedValue(
            mockResponse({
                jsonrpc: '2.0',
                id: '1',
                result: {
                    context: { slot: 50 },
                    value: {
                        items: [
                            {
                                tokenData: {
                                    mint: 'So11111111111111111111111111111111111111112',
                                    owner: 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
                                    amount: '5000',
                                    delegate: null,
                                    state: 'initialized',
                                    tlv: null,
                                },
                                account: {
                                    hash: '11111111111111111111111111111111',
                                    address: null,
                                    data: null,
                                    lamports: '0',
                                    owner: 'cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m',
                                    leafIndex: 3,
                                    seq: null,
                                    slotCreated: '100',
                                    merkleContext: {
                                        tree: 'amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx',
                                        queue: 'SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7',
                                        treeType: TreeType.StateV2,
                                    },
                                    proveByIndex: false,
                                },
                            },
                        ],
                        cursor: null,
                    },
                },
            }),
        );

        const indexer = new PhotonIndexer(ENDPOINT);
        const result = await indexer.getCompressedTokenAccountsByOwner(
            'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA' as any,
        );

        expect(result.value.items).toHaveLength(1);
        expect(result.value.items[0].token.amount).toBe(5000n);
        expect(result.value.items[0].token.state).toBe(1); // AccountState.Initialized
        expect(result.value.cursor).toBeNull();
    });

    it('parses frozen token state correctly', async () => {
        globalThis.fetch = vi.fn().mockResolvedValue(
            mockResponse({
                jsonrpc: '2.0',
                id: '1',
                result: {
                    context: { slot: 50 },
                    value: {
                        items: [
                            {
                                tokenData: {
                                    mint: 'So11111111111111111111111111111111111111112',
                                    owner: 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
                                    amount: '0',
                                    delegate: 'SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7',
                                    state: 'frozen',
                                    tlv: null,
                                },
                                account: {
                                    hash: '11111111111111111111111111111111',
                                    address: null,
                                    data: null,
                                    lamports: '0',
                                    owner: 'cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m',
                                    leafIndex: 0,
                                    seq: null,
                                    slotCreated: '50',
                                    merkleContext: {
                                        tree: 'amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx',
                                        queue: 'SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7',
                                        treeType: TreeType.StateV2,
                                    },
                                    proveByIndex: false,
                                },
                            },
                        ],
                        cursor: null,
                    },
                },
            }),
        );

        const indexer = new PhotonIndexer(ENDPOINT);
        const result = await indexer.getCompressedTokenAccountsByOwner(
            'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA' as any,
        );

        expect(result.value.items[0].token.state).toBe(2); // AccountState.Frozen
        expect(result.value.items[0].token.delegate).not.toBeNull();
    });
});

// ============================================================================
// TESTS: isLightIndexerAvailable
// ============================================================================

describe('isLightIndexerAvailable', () => {
    it('returns true when endpoint is healthy', async () => {
        globalThis.fetch = vi.fn().mockResolvedValue({
            ok: true,
            json: vi.fn().mockResolvedValue({
                jsonrpc: '2.0',
                id: '1',
                result: 'ok',
            }),
        });

        const result = await isLightIndexerAvailable(ENDPOINT);

        expect(result).toBe(true);
    });

    it('returns false when endpoint returns HTTP error', async () => {
        globalThis.fetch = vi.fn().mockResolvedValue({
            ok: false,
            status: 503,
        });

        const result = await isLightIndexerAvailable(ENDPOINT);

        expect(result).toBe(false);
    });

    it('returns false when endpoint returns RPC error', async () => {
        globalThis.fetch = vi.fn().mockResolvedValue({
            ok: true,
            json: vi.fn().mockResolvedValue({
                jsonrpc: '2.0',
                id: '1',
                error: { code: -32000, message: 'Unhealthy' },
            }),
        });

        const result = await isLightIndexerAvailable(ENDPOINT);

        expect(result).toBe(false);
    });

    it('returns false when fetch throws', async () => {
        globalThis.fetch = vi
            .fn()
            .mockRejectedValue(new Error('Network unreachable'));

        const result = await isLightIndexerAvailable(ENDPOINT);

        expect(result).toBe(false);
    });
});
