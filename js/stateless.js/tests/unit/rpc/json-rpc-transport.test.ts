import { afterEach, describe, expect, it, vi } from 'vitest';
import { Connection } from '@solana/web3.js';
import { createRpc, rpcRequest } from '../../../src/rpc';

describe('Photon JSON-RPC transport', () => {
    afterEach(() => vi.unstubAllGlobals());

    const mockResponse = (result: unknown, status = 200) => {
        const fetchMock = vi.fn().mockResolvedValue(
            new Response(JSON.stringify(result), {
                status,
                headers: { 'Content-Type': 'application/json' },
            }),
        );
        vi.stubGlobal('fetch', fetchMock);
        return fetchMock;
    };

    it.each([
        'https://mainnet.helius-rpc.com?api-key=TEST_KEY',
        'https://mainnet.helius-rpc.com/?api-key=TEST_KEY',
        'https://devnet.helius-rpc.com/?api-key=TEST_KEY',
        'https://mainnet.legacy.helius-rpc.com/?api-key=TEST_KEY',
        'http://127.0.0.1:8784',
        'http://127.0.0.1:8784/',
        'https://rpc.example.com/photon?region=eu&api-key=key%2B%2F%3D%26value&tag=one&tag=two',
        'https://rpc.example.com/photon/?region=eu',
    ])('posts to the configured URL unchanged: %s', async endpoint => {
        const fetchMock = mockResponse({
            jsonrpc: '2.0',
            id: 'test-account',
            result: 'ok',
        });

        await expect(createRpc(endpoint).getIndexerHealth()).resolves.toBe(
            'ok',
        );
        expect(fetchMock).toHaveBeenCalledTimes(1);
        expect(fetchMock).toHaveBeenCalledWith(endpoint, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                jsonrpc: '2.0',
                id: 'test-account',
                method: 'getIndexerHealth',
                params: [],
            }),
        });
    });

    it('uses an explicit compression URL with a web3 Connection', async () => {
        const endpoint = 'https://mainnet.helius-rpc.com/?api-key=TEST_KEY';
        const fetchMock = mockResponse({
            jsonrpc: '2.0',
            id: 'test-account',
            result: 123,
        });
        const rpc = createRpc(
            new Connection('http://127.0.0.1:8899'),
            endpoint,
        );

        await expect(rpc.getIndexerSlot()).resolves.toBe(123);
        expect(fetchMock).toHaveBeenCalledTimes(1);
        expect(fetchMock.mock.calls[0][0]).toBe(endpoint);
        expect(JSON.parse(fetchMock.mock.calls[0][1].body).method).toBe(
            'getIndexerSlot',
        );
    });

    it.each<[string, Record<string, unknown>]>([
        ['getCompressedAccount', { hash: '11111111111111111111111111111111' }],
        [
            'getCompressedAccountV2',
            { hash: '11111111111111111111111111111111' },
        ],
        ['getValidityProof', { hashes: [], newAddressesWithTrees: [] }],
        ['getValidityProofV2', { hashes: [], newAddressesWithTrees: [] }],
    ])('carries %s and its parameters in the body', async (method, params) => {
        const endpoint = 'https://mainnet.helius-rpc.com/?api-key=TEST_KEY';
        const response = { jsonrpc: '2.0', id: 'test-account', result: null };
        const fetchMock = mockResponse(response);

        await expect(rpcRequest(endpoint, method, params)).resolves.toEqual(
            response,
        );
        expect(fetchMock).toHaveBeenCalledTimes(1);
        expect(fetchMock.mock.calls[0][0]).toBe(endpoint);
        expect(JSON.parse(fetchMock.mock.calls[0][1].body)).toEqual({
            jsonrpc: '2.0',
            id: 'test-account',
            method,
            params,
        });
    });

    it('surfaces JSON-RPC errors without retrying a REST or legacy URL', async () => {
        const fetchMock = mockResponse({
            jsonrpc: '2.0',
            id: 'test-account',
            error: { code: -32601, message: 'Method not found' },
        });

        await expect(
            createRpc(
                'https://mainnet.helius-rpc.com/?api-key=TEST_KEY',
            ).getIndexerHealth(),
        ).rejects.toThrow('Method not found');
        expect(fetchMock).toHaveBeenCalledTimes(1);
    });

    it('surfaces HTTP errors', async () => {
        const fetchMock = mockResponse({ error: 'Unauthorized' }, 401);

        await expect(
            createRpc(
                'https://mainnet.helius-rpc.com/?api-key=TEST_KEY',
            ).getIndexerHealth(),
        ).rejects.toThrow('HTTP error! status: 401');
        expect(fetchMock).toHaveBeenCalledTimes(1);
    });
});
