import { describe, it, expect } from 'vitest';
import {
    toOffChainMetadataJson,
    OffChainTokenMetadata,
    OffChainTokenMetadataJson,
} from '../../src/v3/layout/layout-token-metadata';

describe('layout-token-metadata', () => {
    describe('toOffChainMetadataJson', () => {
        it('should convert basic metadata', () => {
            const meta: OffChainTokenMetadata = {
                name: 'My Token',
                symbol: 'MTK',
            };

            const json = toOffChainMetadataJson(meta);

            expect(json.name).toBe('My Token');
            expect(json.symbol).toBe('MTK');
            expect(json.description).toBeUndefined();
            expect(json.image).toBeUndefined();
            expect(json.additionalMetadata).toBeUndefined();
        });

        it('should include description when provided', () => {
            const meta: OffChainTokenMetadata = {
                name: 'Described Token',
                symbol: 'DESC',
                description: 'A token with a description',
            };

            const json = toOffChainMetadataJson(meta);

            expect(json.name).toBe('Described Token');
            expect(json.symbol).toBe('DESC');
            expect(json.description).toBe('A token with a description');
        });

        it('should include image when provided', () => {
            const meta: OffChainTokenMetadata = {
                name: 'Image Token',
                symbol: 'IMG',
                image: 'https://example.com/token.png',
            };

            const json = toOffChainMetadataJson(meta);

            expect(json.image).toBe('https://example.com/token.png');
        });

        it('should include additional metadata when provided', () => {
            const meta: OffChainTokenMetadata = {
                name: 'Extended Token',
                symbol: 'EXT',
                additionalMetadata: [
                    { key: 'version', value: '1.0' },
                    { key: 'creator', value: 'Light Protocol' },
                ],
            };

            const json = toOffChainMetadataJson(meta);

            expect(json.additionalMetadata).toBeDefined();
            expect(json.additionalMetadata?.length).toBe(2);
            expect(json.additionalMetadata?.[0]).toEqual({
                key: 'version',
                value: '1.0',
            });
            expect(json.additionalMetadata?.[1]).toEqual({
                key: 'creator',
                value: 'Light Protocol',
            });
        });

        it('should not include additionalMetadata when array is empty', () => {
            const meta: OffChainTokenMetadata = {
                name: 'No Extra',
                symbol: 'NEX',
                additionalMetadata: [],
            };

            const json = toOffChainMetadataJson(meta);

            expect(json.additionalMetadata).toBeUndefined();
        });

        it('should include all fields when all provided', () => {
            const meta: OffChainTokenMetadata = {
                name: 'Complete Token',
                symbol: 'COMP',
                description: 'A complete token with all metadata fields',
                image: 'https://example.com/complete.png',
                additionalMetadata: [
                    { key: 'website', value: 'https://complete.com' },
                ],
            };

            const json = toOffChainMetadataJson(meta);

            expect(json.name).toBe('Complete Token');
            expect(json.symbol).toBe('COMP');
            expect(json.description).toBe(
                'A complete token with all metadata fields',
            );
            expect(json.image).toBe('https://example.com/complete.png');
            expect(json.additionalMetadata?.length).toBe(1);
        });

        it('should handle empty strings', () => {
            const meta: OffChainTokenMetadata = {
                name: '',
                symbol: '',
                description: '',
                image: '',
            };

            const json = toOffChainMetadataJson(meta);

            expect(json.name).toBe('');
            expect(json.symbol).toBe('');
            // Empty strings are still included (not undefined)
            expect(json.description).toBe('');
            expect(json.image).toBe('');
        });

        it('should handle unicode characters', () => {
            const meta: OffChainTokenMetadata = {
                name: 'Token',
                symbol: '$TKN',
                description: 'Unicode: cafe, 100, ',
            };

            const json = toOffChainMetadataJson(meta);

            expect(json.name).toBe('Token');
            expect(json.symbol).toBe('$TKN');
            expect(json.description).toBe('Unicode: cafe, 100, ');
        });

        it('should handle long strings', () => {
            const longName = 'A'.repeat(1000);
            const longDescription = 'B'.repeat(5000);
            const longUri = 'https://example.com/' + 'C'.repeat(500);

            const meta: OffChainTokenMetadata = {
                name: longName,
                symbol: 'LONG',
                description: longDescription,
                image: longUri,
            };

            const json = toOffChainMetadataJson(meta);

            expect(json.name.length).toBe(1000);
            expect(json.description?.length).toBe(5000);
            // 'https://example.com/' is 20 chars + 500 'C's = 520
            expect(json.image?.length).toBe(520);
        });

        it('should handle special characters in metadata', () => {
            const meta: OffChainTokenMetadata = {
                name: 'Token <with> "special" chars & more',
                symbol: 'SPC',
                additionalMetadata: [
                    { key: 'json-key', value: '{"nested": "json"}' },
                ],
            };

            const json = toOffChainMetadataJson(meta);

            expect(json.name).toBe('Token <with> "special" chars & more');
            expect(json.additionalMetadata?.[0].value).toBe(
                '{"nested": "json"}',
            );
        });

        it('should be JSON serializable', () => {
            const meta: OffChainTokenMetadata = {
                name: 'Serializable',
                symbol: 'SER',
                description: 'Can be converted to JSON string',
                image: 'https://example.com/ser.png',
                additionalMetadata: [{ key: 'test', value: 'value' }],
            };

            const json = toOffChainMetadataJson(meta);

            // Should not throw
            const jsonString = JSON.stringify(json);
            const parsed = JSON.parse(jsonString) as OffChainTokenMetadataJson;

            expect(parsed.name).toBe('Serializable');
            expect(parsed.symbol).toBe('SER');
            expect(parsed.description).toBe('Can be converted to JSON string');
            expect(parsed.additionalMetadata?.[0]).toEqual({
                key: 'test',
                value: 'value',
            });
        });
    });
});
