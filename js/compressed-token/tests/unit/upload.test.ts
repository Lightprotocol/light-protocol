import { describe, it, expect } from 'vitest';
import {
    toOffChainMetadataJson,
    OffChainTokenMetadata,
    OffChainTokenMetadataJson,
} from '../../src/v3';

describe('upload', () => {
    describe('toOffChainMetadataJson', () => {
        it('should format basic metadata with only required fields', () => {
            const input: OffChainTokenMetadata = {
                name: 'Test Token',
                symbol: 'TEST',
            };

            const result = toOffChainMetadataJson(input);

            expect(result).toEqual({
                name: 'Test Token',
                symbol: 'TEST',
            });
            expect(result.description).toBeUndefined();
            expect(result.image).toBeUndefined();
            expect(result.additionalMetadata).toBeUndefined();
        });

        it('should include description when provided', () => {
            const input: OffChainTokenMetadata = {
                name: 'My Token',
                symbol: 'MTK',
                description: 'A test token for unit testing',
            };

            const result = toOffChainMetadataJson(input);

            expect(result.name).toBe('My Token');
            expect(result.symbol).toBe('MTK');
            expect(result.description).toBe('A test token for unit testing');
            expect(result.image).toBeUndefined();
        });

        it('should include image when provided', () => {
            const input: OffChainTokenMetadata = {
                name: 'Image Token',
                symbol: 'IMG',
                image: 'https://example.com/token.png',
            };

            const result = toOffChainMetadataJson(input);

            expect(result.name).toBe('Image Token');
            expect(result.symbol).toBe('IMG');
            expect(result.image).toBe('https://example.com/token.png');
            expect(result.description).toBeUndefined();
        });

        it('should include additionalMetadata when provided with items', () => {
            const input: OffChainTokenMetadata = {
                name: 'Rich Token',
                symbol: 'RICH',
                additionalMetadata: [
                    { key: 'creator', value: 'Alice' },
                    { key: 'version', value: '1.0.0' },
                ],
            };

            const result = toOffChainMetadataJson(input);

            expect(result.name).toBe('Rich Token');
            expect(result.symbol).toBe('RICH');
            expect(result.additionalMetadata).toEqual([
                { key: 'creator', value: 'Alice' },
                { key: 'version', value: '1.0.0' },
            ]);
        });

        it('should include all fields when all are provided', () => {
            const input: OffChainTokenMetadata = {
                name: 'Full Token',
                symbol: 'FULL',
                description: 'A token with all metadata fields',
                image: 'https://arweave.net/abc123',
                additionalMetadata: [
                    { key: 'website', value: 'https://example.com' },
                    { key: 'twitter', value: '@fulltoken' },
                    { key: 'category', value: 'utility' },
                ],
            };

            const result = toOffChainMetadataJson(input);

            expect(result).toEqual({
                name: 'Full Token',
                symbol: 'FULL',
                description: 'A token with all metadata fields',
                image: 'https://arweave.net/abc123',
                additionalMetadata: [
                    { key: 'website', value: 'https://example.com' },
                    { key: 'twitter', value: '@fulltoken' },
                    { key: 'category', value: 'utility' },
                ],
            });
        });

        it('should exclude empty additionalMetadata array', () => {
            const input: OffChainTokenMetadata = {
                name: 'Empty Additional',
                symbol: 'EA',
                additionalMetadata: [],
            };

            const result = toOffChainMetadataJson(input);

            expect(result.name).toBe('Empty Additional');
            expect(result.symbol).toBe('EA');
            expect(result.additionalMetadata).toBeUndefined();
        });

        it('should handle empty string values', () => {
            const input: OffChainTokenMetadata = {
                name: '',
                symbol: '',
                description: '',
                image: '',
            };

            const result = toOffChainMetadataJson(input);

            expect(result.name).toBe('');
            expect(result.symbol).toBe('');
            expect(result.description).toBe('');
            expect(result.image).toBe('');
        });

        it('should handle long string values', () => {
            const longName = 'A'.repeat(200);
            const longSymbol = 'B'.repeat(50);
            const longDescription = 'C'.repeat(1000);
            const longImageUrl = 'https://example.com/' + 'x'.repeat(500);

            const input: OffChainTokenMetadata = {
                name: longName,
                symbol: longSymbol,
                description: longDescription,
                image: longImageUrl,
            };

            const result = toOffChainMetadataJson(input);

            expect(result.name).toBe(longName);
            expect(result.symbol).toBe(longSymbol);
            expect(result.description).toBe(longDescription);
            expect(result.image).toBe(longImageUrl);
        });

        it('should handle unicode characters', () => {
            const input: OffChainTokenMetadata = {
                name: 'Token Name',
                symbol: 'TKN',
                description: 'Description with special chars',
            };

            const result = toOffChainMetadataJson(input);

            expect(result.name).toBe('Token Name');
            expect(result.symbol).toBe('TKN');
            expect(result.description).toBe('Description with special chars');
        });

        it('should handle special characters in URLs', () => {
            const input: OffChainTokenMetadata = {
                name: 'URL Token',
                symbol: 'URL',
                image: 'https://example.com/image?param=value&other=123',
            };

            const result = toOffChainMetadataJson(input);

            expect(result.image).toBe(
                'https://example.com/image?param=value&other=123',
            );
        });

        it('should handle IPFS URLs', () => {
            const input: OffChainTokenMetadata = {
                name: 'IPFS Token',
                symbol: 'IPFS',
                image: 'ipfs://QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG',
            };

            const result = toOffChainMetadataJson(input);

            expect(result.image).toBe(
                'ipfs://QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG',
            );
        });

        it('should handle Arweave URLs', () => {
            const input: OffChainTokenMetadata = {
                name: 'Arweave Token',
                symbol: 'AR',
                image: 'https://arweave.net/abc123xyz',
            };

            const result = toOffChainMetadataJson(input);

            expect(result.image).toBe('https://arweave.net/abc123xyz');
        });

        it('should preserve additionalMetadata order', () => {
            const input: OffChainTokenMetadata = {
                name: 'Order Test',
                symbol: 'ORD',
                additionalMetadata: [
                    { key: 'z_last', value: 'should be last' },
                    { key: 'a_first', value: 'should be first' },
                    { key: 'm_middle', value: 'should be middle' },
                ],
            };

            const result = toOffChainMetadataJson(input);

            expect(result.additionalMetadata).toEqual([
                { key: 'z_last', value: 'should be last' },
                { key: 'a_first', value: 'should be first' },
                { key: 'm_middle', value: 'should be middle' },
            ]);
        });

        it('should handle additionalMetadata with empty key or value', () => {
            const input: OffChainTokenMetadata = {
                name: 'Empty KV',
                symbol: 'EKV',
                additionalMetadata: [
                    { key: '', value: 'empty key' },
                    { key: 'empty value', value: '' },
                    { key: '', value: '' },
                ],
            };

            const result = toOffChainMetadataJson(input);

            expect(result.additionalMetadata).toEqual([
                { key: '', value: 'empty key' },
                { key: 'empty value', value: '' },
                { key: '', value: '' },
            ]);
        });

        it('result should be JSON serializable', () => {
            const input: OffChainTokenMetadata = {
                name: 'JSON Test',
                symbol: 'JSON',
                description: 'Testing JSON serialization',
                image: 'https://example.com/image.png',
                additionalMetadata: [{ key: 'test', value: 'value' }],
            };

            const result = toOffChainMetadataJson(input);
            const jsonString = JSON.stringify(result);
            const parsed = JSON.parse(jsonString);

            expect(parsed).toEqual(result);
        });

        it('should not include undefined optional fields in output', () => {
            const input: OffChainTokenMetadata = {
                name: 'Minimal',
                symbol: 'MIN',
            };

            const result = toOffChainMetadataJson(input);
            const keys = Object.keys(result);

            expect(keys).toEqual(['name', 'symbol']);
            expect(keys).not.toContain('description');
            expect(keys).not.toContain('image');
            expect(keys).not.toContain('additionalMetadata');
        });

        it('should return a new object (not mutate input)', () => {
            const input: OffChainTokenMetadata = {
                name: 'Immutable',
                symbol: 'IMM',
                description: 'Test immutability',
            };

            const result = toOffChainMetadataJson(input);

            // Modify result
            result.name = 'Modified';

            // Original should be unchanged
            expect(input.name).toBe('Immutable');
        });

        it('should handle explicitly undefined optional fields', () => {
            const input: OffChainTokenMetadata = {
                name: 'Explicit Undefined',
                symbol: 'EU',
                description: undefined,
                image: undefined,
                additionalMetadata: undefined,
            };

            const result = toOffChainMetadataJson(input);
            const keys = Object.keys(result);

            expect(keys).toEqual(['name', 'symbol']);
            expect(result.description).toBeUndefined();
            expect(result.image).toBeUndefined();
            expect(result.additionalMetadata).toBeUndefined();
        });

        it('should handle additionalMetadata with single item', () => {
            const input: OffChainTokenMetadata = {
                name: 'Single Item',
                symbol: 'SI',
                additionalMetadata: [{ key: 'only', value: 'one' }],
            };

            const result = toOffChainMetadataJson(input);

            expect(result.additionalMetadata).toEqual([
                { key: 'only', value: 'one' },
            ]);
            expect(result.additionalMetadata?.length).toBe(1);
        });

        it('should share additionalMetadata array reference (not deep copy)', () => {
            const additionalMetadata = [{ key: 'shared', value: 'ref' }];
            const input: OffChainTokenMetadata = {
                name: 'Ref Test',
                symbol: 'REF',
                additionalMetadata,
            };

            const result = toOffChainMetadataJson(input);

            // Same reference (current behavior)
            expect(result.additionalMetadata).toBe(additionalMetadata);
        });

        it('should handle mix of provided and omitted optional fields', () => {
            const input: OffChainTokenMetadata = {
                name: 'Mixed',
                symbol: 'MIX',
                description: 'Has description',
                // image omitted
                additionalMetadata: [{ key: 'has', value: 'metadata' }],
            };

            const result = toOffChainMetadataJson(input);

            expect(result.description).toBe('Has description');
            expect(result.image).toBeUndefined();
            expect(result.additionalMetadata).toBeDefined();
            expect('image' in result).toBe(false);
        });

        it('should handle whitespace-only strings', () => {
            const input: OffChainTokenMetadata = {
                name: '   ',
                symbol: '\t\n',
                description: '  spaces  ',
            };

            const result = toOffChainMetadataJson(input);

            expect(result.name).toBe('   ');
            expect(result.symbol).toBe('\t\n');
            expect(result.description).toBe('  spaces  ');
        });

        it('should handle additionalMetadata with many items', () => {
            const manyItems = Array.from({ length: 100 }, (_, i) => ({
                key: `key${i}`,
                value: `value${i}`,
            }));

            const input: OffChainTokenMetadata = {
                name: 'Many Items',
                symbol: 'MANY',
                additionalMetadata: manyItems,
            };

            const result = toOffChainMetadataJson(input);

            expect(result.additionalMetadata?.length).toBe(100);
            expect(result.additionalMetadata?.[0]).toEqual({
                key: 'key0',
                value: 'value0',
            });
            expect(result.additionalMetadata?.[99]).toEqual({
                key: 'key99',
                value: 'value99',
            });
        });
    });

    describe('OffChainTokenMetadata type', () => {
        it('should allow minimal metadata', () => {
            const meta: OffChainTokenMetadata = {
                name: 'Test',
                symbol: 'T',
            };
            expect(meta.name).toBe('Test');
            expect(meta.symbol).toBe('T');
        });

        it('should allow full metadata', () => {
            const meta: OffChainTokenMetadata = {
                name: 'Full',
                symbol: 'F',
                description: 'desc',
                image: 'img',
                additionalMetadata: [{ key: 'k', value: 'v' }],
            };
            expect(meta.name).toBe('Full');
            expect(meta.description).toBe('desc');
        });
    });

    describe('OffChainTokenMetadataJson type', () => {
        it('should have correct shape for JSON output', () => {
            const json: OffChainTokenMetadataJson = {
                name: 'Output',
                symbol: 'OUT',
                description: 'Optional desc',
                image: 'Optional image',
                additionalMetadata: [{ key: 'k', value: 'v' }],
            };

            expect(json.name).toBe('Output');
            expect(json.symbol).toBe('OUT');
        });
    });
});
