import { describe, it, expect } from 'vitest';
import {
    CompressedTokenAccountsByOwnerOrDelegateResult,
    jsonRpcResultAndContext,
    toCamelCase,
} from '../../src';
import { create } from 'superstruct';

var JSONbig = require('json-bigint');

describe('safely convert json response', async () => {
    it('should convert unsafe integer responses safely', async () => {
        const rawResponse = `{
                "jsonrpc": "2.0",
                "result": {
                    "context": { "slot": 320706183 },
                    "value": {
                        "items": [
                            {
                                "account": {
                                    "hash": "KtkksPFTArx5iU4cfLgqM9jsf1cee7J8uDNDM8doPgr",
                                    "address": null,
                                    "data": {
                                        "discriminator": 2,
                                        "data": "1dIGwA1L2D6JdM7t5Gp6/GcuEzbicc2B1Y83y0aN1D+kOH8fxBZRU5LxMLe0XqySi6MXxORqVur8NEGW2l3m4ZCqrNzaIseKAAAA",
                                        "dataHash": "2TWLSZWJrrmmuZD9NGBbKNnEyAEBsAH4CC8HJF5NLJ1V"
                                    },
                                    "owner": "cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m",
                                    "lamports": 0,
                                    "tree": "smt5uPaQT9n6b1qAkgyonmzRxtuazA53Rddwntqistc",
                                    "leafIndex": 73722,
                                    "seq": 87594,
                                    "slotCreated": 320438700
                                },
                                "tokenData": {
                                    "mint": "FPfb74MgXL3Gj1jhkSKactfsiqgiJ162ZcswnuTnUzGE",
                                    "owner": "C43q3Nx7KgGCPX8byv4w4Apq48n7iuBV4wpGsepArHu2",
                                    "amount": 9999999820999994000,
                                    "delegate": null,
                                    "state": "initialized",
                                    "tlv": null
                                }
                            }
                        ],
                        "cursor": null
                    }
                },
                "id": "test-account"
            }`;

        const mockRes = {
            text: async () => rawResponse,
        };

        // this is replicating RPC internals
        const text = await mockRes.text();
        const resParsed = JSONbig.parse(text);
        const unsafeRes = toCamelCase(resParsed);
        expect(unsafeRes.result.value.items[0].account.data.discriminator).toBe(
            2,
        );

        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(
                CompressedTokenAccountsByOwnerOrDelegateResult,
            ),
        );

        if ('error' in res) {
            throw new Error('error in res');
        }
        if (res.result.value === null) {
            throw new Error('not implemented: NULL result');
        }

        const tokenAmount = res.result.value.items[0].tokenData.amount;

        expect(tokenAmount.toString()).toBe('9999999820999994000');
    });

    it('should convert small integer responses correctly', async () => {
        const rawResponse = `{
                "jsonrpc": "2.0",
                "result": {
                    "context": { "slot": 320706183 },
                    "value": {
                        "items": [
                            {
                                "account": {
                                    "hash": "KtkksPFTArx5iU4cfLgqM9jsf1cee7J8uDNDM8doPgr",
                                    "address": null,
                                    "data": {
                                        "discriminator": 2,
                                        "data": "1dIGwA1L2D6JdM7t5Gp6/GcuEzbicc2B1Y83y0aN1D+kOH8fxBZRU5LxMLe0XqySi6MXxORqVur8NEGW2l3m4ZCqrNzaIseKAAAA",
                                        "dataHash": "2TWLSZWJrrmmuZD9NGBbKNnEyAEBsAH4CC8HJF5NLJ1V"
                                    },
                                    "owner": "cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m",
                                    "lamports": 0,
                                    "tree": "smt5uPaQT9n6b1qAkgyonmzRxtuazA53Rddwntqistc",
                                    "leafIndex": 73722,
                                    "seq": 87594,
                                    "slotCreated": 320438700
                                },
                                "tokenData": {
                                    "mint": "FPfb74MgXL3Gj1jhkSKactfsiqgiJ162ZcswnuTnUzGE",
                                    "owner": "C43q3Nx7KgGCPX8byv4w4Apq48n7iuBV4wpGsepArHu2",
                                    "amount": 999999982,
                                    "delegate": null,
                                    "state": "initialized",
                                    "tlv": null
                                }
                            }
                        ],
                        "cursor": null
                    }
                },
                "id": "test-account"
            }`;

        const mockRes = {
            text: async () => rawResponse,
        };

        // this is replicating RPC internals
        const text = await mockRes.text();
        const resParsed = JSONbig.parse(text);
        const unsafeRes = toCamelCase(resParsed);
        expect(unsafeRes.result.value.items[0].account.data.discriminator).toBe(
            2,
        );

        const res = create(
            unsafeRes,
            jsonRpcResultAndContext(
                CompressedTokenAccountsByOwnerOrDelegateResult,
            ),
        );

        if ('error' in res) {
            throw new Error('error in res');
        }
        if (res.result.value === null) {
            throw new Error('not implemented: NULL result');
        }

        const tokenAmount = res.result.value.items[0].tokenData.amount;

        expect(tokenAmount.toString()).toBe('999999982');
    });
});
