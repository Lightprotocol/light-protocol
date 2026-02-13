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
            json: vi.fn().mockRejectedValue(new SyntaxError('Unexpected token')),
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
        globalThis.fetch = vi.fn().mockResolvedValue({
            ok: true,
            json: vi.fn().mockResolvedValue({
                jsonrpc: '2.0',
                id: '1',
                error: { code: -32600, message: 'Invalid' },
            }),
        });

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
        globalThis.fetch = vi.fn().mockResolvedValue({
            ok: true,
            json: vi.fn().mockResolvedValue({
                jsonrpc: '2.0',
                id: '1',
            }),
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
            expect((e as IndexerError).message).toContain('Missing result');
        }
    });

    it('throws IndexerError for V1 tree type in account response', async () => {
        globalThis.fetch = vi.fn().mockResolvedValue({
            ok: true,
            json: vi.fn().mockResolvedValue({
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
            expect((e as IndexerError).message).toContain(
                'V1 tree types are not supported',
            );
        }
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
