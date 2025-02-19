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

            it('should handle boundary values correctly', () => {
                const input1 = '{"value": 9007199254740991}';
                const output1 = wrapBigNumbersAsStrings(input1);
                expect(output1).to.equal('{"value": 9007199254740991}');

                const input2 = '{"value": 9007199254740992}';
                const output2 = wrapBigNumbersAsStrings(input2);
                expect(output2).to.equal('{"value": "9007199254740992"}');

                const input3 = '{"value": -9007199254740991}';
                const output3 = wrapBigNumbersAsStrings(input3);
                expect(output3).to.equal('{"value": -9007199254740991}');

                const input4 = '{"value": -9007199254740992}';
                const output4 = wrapBigNumbersAsStrings(input4);
                expect(output4).to.equal('{"value": "-9007199254740992"}');
            });

            it('should not alter non-numeric values', () => {
                const input = '{"value": "12345678901234567890"}';
                const output = wrapBigNumbersAsStrings(input);
                expect(output).to.equal('{"value": "12345678901234567890"}');
            });

            it('should handle mixed content correctly', () => {
                const input =
                    '{"value": 123, "text": "hello", "big": 9007199254740992}';
                const output = wrapBigNumbersAsStrings(input);
                expect(output).to.equal(
                    '{"value": 123, "text": "hello", "big": "9007199254740992"}',
                );
            });

            it('should handle arrays of numbers correctly', () => {
                const input = '{"values": [1, 9007199254740992, 3]}';
                const output = wrapBigNumbersAsStrings(input);
                expect(output).to.equal(
                    '{"values": [1, "9007199254740992", 3]}',
                );
            });

            it('should handle nested objects correctly', () => {
                const input = '{"outer": {"inner": 9007199254740992}}';
                const output = wrapBigNumbersAsStrings(input);
                expect(output).to.equal(
                    '{"outer": {"inner": "9007199254740992"}}',
                );
            });

            it('should handle negative numbers beyond safe integer limits', () => {
                const input = '{"value": -9007199254740993}';
                const output = wrapBigNumbersAsStrings(input);
                expect(output).to.equal('{"value": "-9007199254740993"}');
            });

            it('should not wrap zero and small numbers', () => {
                const input1 = '{"value": 0}';
                const output1 = wrapBigNumbersAsStrings(input1);
                expect(output1).to.equal('{"value": 0}');

                const input2 = '{"value": 42}';
                const output2 = wrapBigNumbersAsStrings(input2);
                expect(output2).to.equal('{"value": 42}');
            });

            it('should handle edge case with trailing comma', () => {
                const input = '{"value": 9007199254740992,}';
                const output = wrapBigNumbersAsStrings(input);
                expect(output).to.equal('{"value": "9007199254740992",}');
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
                                "hash": "u2JbcatZ1LwQTAeqhtKebDQz6rpoynY7T7xMED75SkS",
                                "address": null,
                                "data": {
                                "discriminator": 2,
                                "data": "NF3Ab9WWSk3xXineM02bJDYrYxrT81Ic+honfH21uPrRZHI+RYopNP+1f7oYgg5ZnC5+yaxEh7KPpQVdvMquQTIAAAAAAAAAAAAA",
                                "dataHash": "3FR7ziRtG5SadCZfuZJ4unHaqQiUUvH37wKTrd2WpWfH"
                                },
                                "owner": "cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m",
                                "lamports": 0,
                                "tree": "smt1NamzXdq4AMqS2fS2F1i5KTYPZRhoHgWx38d8WsT",
                                "leafIndex": 56910290,
                                "seq": 57043968,
                                "slotCreated": 320678665
                            },
                            "tokenData": {
                                "mint": "4XR7f5d3SyK7MpJ5Qk96HitRyV7x3AikJGXug88YbkPK",
                                "owner": "F6P3Z54AG7qj9bBn5MFDZ63V6mejxRRY7QVThnJzmUhN",
                                "amount": 50492492432742222242222,
                                "delegate": null,
                                "state": "initialized",
                                "tlv": null
                            }
                        },
                        {
                            "account": {
                                "hash": "2KMeKsaJq6uJwXbgBmEUB9GjtXgcPYqfeBUCDyHn5sp2",
                                "address": null,
                                "data": {
                                "discriminator": 2,
                                "data": "NF3Ab9WWSk3xXineM02bJDYrYxrT81Ic+honfH21uPrRZHI+RYopNP+1f7oYgg5ZnC5+yaxEh7KPpQVdvMquQTIAAAAAAAAAAAAA",
                                "dataHash": "3FR7ziRtG5SadCZfuZJ4unHaqQiUUvH37wKTrd2WpWfH"
                                },
                                "owner": "cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m",
                                "lamports": 0,
                                "tree": "smt6ukQDSPPYHSshQovmiRUjG9jGFq2hW9vgrDFk5Yz",
                                "leafIndex": 72303,
                                "seq": 86934,
                                "slotCreated": 320679057
                            },
                            "tokenData": {
                                "mint": "4XR7f5d3SyK7MpJ5Qk96HitRyV7x3AikJGXug88YbkPK",
                                "owner": "F6P3Z54AG7qj9bBn5MFDZ63V6mejxRRY7QVThnJzmUhN",
                                "amount": 50,
                                "delegate": null,
                                "state": "initialized",
                                "tlv": null
                            }
                        },
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
                        },
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
                                "amount": 18446744073709551615,
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
        expect(unsafeRes.result.value.items[1].account.data.discriminator).toBe(
            2,
        );
        expect(unsafeRes.result.value.items[2].account.data.discriminator).toBe(
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

        const tokenAmount0 = res.result.value.items[0].tokenData.amount;
        const tokenAmount1 = res.result.value.items[1].tokenData.amount;
        const tokenAmount2 = res.result.value.items[2].tokenData.amount;
        const tokenAmount3 = res.result.value.items[3].tokenData.amount;

        expect(tokenAmount0.toString()).toBe('50492492432742222242222');
        expect(tokenAmount1.toString()).toBe('50');
        expect(tokenAmount2.toString()).toBe('24000000000000050');
        expect(tokenAmount3.toString()).toBe('18446744073709551615');
    });
});
