import { describe, it, expect } from 'vitest';
import {
    CompressedTokenAccountsByOwnerOrDelegateResult,
    jsonRpcResultAndContext,
    toCamelCase,
    wrapBigNumbersAsStrings,
} from '../../src';
import { create } from 'superstruct';
import { BN } from 'bn.js';

describe('safely convert json response', async () => {
    it('should stringify correctly', () => {
        describe('wrapBigNumbersAsStrings', () => {
            it('should wrap large numbers as strings', () => {
                const input = '{"value": 9007199254740992}';
                const output = wrapBigNumbersAsStrings(input);
                expect(output).to.equal('{"value": "9007199254740992"}');
            });

            it('should wrap small numbers as strings', () => {
                const input = '{"value": -9007199254740992}';
                const output = wrapBigNumbersAsStrings(input);
                expect(output).to.equal('{"value": "-9007199254740992"}');
            });

            it('should not wrap safe numbers', () => {
                const input = '{"value": 100}';
                const output = wrapBigNumbersAsStrings(input);
                expect(output).to.equal('{"value": 100}');
            });

            it('should handle multiple numbers', () => {
                const input = '{"a": 9007199254740992, "b": 100}';
                const output = wrapBigNumbersAsStrings(input);
                expect(output).to.equal('{"a": "9007199254740992", "b": 100}');
            });

            it('should handle numbers in arrays', () => {
                const input = '{"values": [9007199254740992, 100]}';
                const output = wrapBigNumbersAsStrings(input);
                expect(output).to.equal(
                    '{"values": ["9007199254740992", 100]}',
                );
            });

            it('should not alter non-numeric strings', () => {
                const input = '{"value": "string"}';
                const output = wrapBigNumbersAsStrings(input);
                expect(output).to.equal('{"value": "string"}');
            });

            it('should not alter mixed strings', () => {
                const input = '{"value": "ejed2323"}';
                const output = wrapBigNumbersAsStrings(input);
                expect(output).to.equal('{"value": "ejed2323"}');
            });

            it('should wrap negative small numbers as strings', () => {
                const input = '{"value": -100}';
                const output = wrapBigNumbersAsStrings(input);
                expect(output).to.equal('{"value": -100}');
            });
        });
    });
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
        const wrappedJsonString = wrapBigNumbersAsStrings(text);

        const resParsed = JSON.parse(wrappedJsonString);
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

        if ('error' in res) throw new Error('error in res');
        if (res.result.value === null)
            throw new Error('not implemented: NULL result');
        expect(
            res.result.value.items[0].account.data!.discriminator,
        ).toStrictEqual(new BN(2));
        const tokenAmount = res.result.value.items[0].tokenData.amount;

        expect(tokenAmount.toString()).toBe('9999999820999994000');
    });

    it('should convert unsafe integer responses safely 2', async () => {
        const rawResponse = `{
                "jsonrpc": "2.0",
                "result": {
                    "context": { "slot": 320861323 },
                    "value": {
                        "items": [
                            {
                                "account": {
                                    "hash": "3YQ9Z5WDKmAhRPhTLXSgWFF7PmvEc2wCzu1S9ecdRpfq",
                                    "address": null,
                                    "data": {
                                        "discriminator": 2,
                                        "data": "thog/y4lF2C9sz186nyHoRF7+XC8KkiOgJ88Buyar6trPSjaV5hSAPSpMDPYSdh7rIGRQK4PvBP0avFGzPD7ADEhbmuchlUAAAAA",
                                        "dataHash": "V1nn5Whtp2jrc4kBZySmLu6FrQXB4ueqpxqZCWpQsim"
                                    },
                                    "owner": "cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m",
                                    "lamports": 0,
                                    "tree": "smt1NamzXdq4AMqS2fS2F1i5KTYPZRhoHgWx38d8WsT",
                                    "leafIndex": 46382363,
                                    "seq": 46497075,
                                    "slotCreated": 318579332
                                },
                                "tokenData": {
                                    "mint": "DFrJxDoLMYt6bNYeNe8Wrjzj2UPUSLZLEMMYBLuTKcTk",
                                    "owner": "8DciPEHkzpgphzrmwQLjz6PAGhkKePJjhy2MnKmNWPCK",
                                    "amount": 24073379395805489,
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
        const wrappedJsonString = wrapBigNumbersAsStrings(text);

        const resParsed = JSON.parse(wrappedJsonString);
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

        expect(tokenAmount.toString()).toBe('24073379395805489');
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
        const wrappedJsonString = wrapBigNumbersAsStrings(text);
        const resParsed = JSON.parse(wrappedJsonString);
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

    it('should convert', async () => {
        const rawResponse = `{
            "jsonrpc": "2.0",
            "result": {
                "context": { "slot": 320850194 },
                "value": {
                    "items": [
                        {
                            "account": {
                                "hash": "34ovj4DVU1qCFKqsiPrYz4zK58Sze6Cg2RwQD22eb5UB",
                                "address": null,
                                "data": {
                                    "discriminator": 2,
                                    "data": "NF3Ab9WWSk3xXineM02bJDYrYxrT81Ic+honfH21uPrRZHI+RYopNP+1f7oYgg5ZnC5+yaxEh7KPpQVdvMquQTIAnHLfQ1UAAAAA",
                                    "dataHash": "FK438LjvH1CmsV7bLBYZudzv4XB2xy7SkcfjJSVn2xg"
                                },
                                "owner": "cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m",
                                "lamports": 0,
                                "tree": "smt9ReAYRF5eFjTd5gBJMn5aKwNRcmp3ub2CQr2vW7j",
                                "leafIndex": 72930,
                                "seq": 87904,
                                "slotCreated": 320848741
                            },
                            "tokenData": {
                                "mint": "4XR7f5d3SyK7MpJ5Qk96HitRyV7x3AikJGXug88YbkPK",
                                "owner": "F6P3Z54AG7qj9bBn5MFDZ63V6mejxRRY7QVThnJzmUhN",
                                "amount": 24000000000000050,
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

        const wrappedJsonString = wrapBigNumbersAsStrings(text);

        const resParsed = JSON.parse(wrappedJsonString);
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

        expect(tokenAmount.toString()).toBe('24000000000000050');
    });
});
