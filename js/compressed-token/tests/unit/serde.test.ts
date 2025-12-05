import { describe, it, expect } from 'vitest';
import { PublicKey, Keypair } from '@solana/web3.js';
import { Buffer } from 'buffer';
import {
    deserializeMint,
    serializeMint,
    decodeTokenMetadata,
    encodeTokenMetadata,
    extractTokenMetadata,
    parseTokenMetadata,
    toMintInstructionData,
    toMintInstructionDataWithMetadata,
    CompressedMint,
    BaseMint,
    MintContext,
    MintExtension,
    TokenMetadata,
    MintInstructionData,
    MintInstructionDataWithMetadata,
    MintMetadataField,
    ExtensionType,
    MINT_CONTEXT_SIZE,
    MintContextLayout,
} from '../../src/v3';
import { MINT_SIZE } from '@solana/spl-token';

describe('serde', () => {
    describe('MintContextLayout', () => {
        it('should have correct size (34 bytes)', () => {
            expect(MINT_CONTEXT_SIZE).toBe(34);
            expect(MintContextLayout.span).toBe(34);
        });
    });

    describe('deserializeMint / serializeMint roundtrip', () => {
        const testCases: { description: string; mint: CompressedMint }[] = [
            {
                description: 'minimal mint without extensions',
                mint: {
                    base: {
                        mintAuthority: null,
                        supply: BigInt(0),
                        decimals: 9,
                        isInitialized: true,
                        freezeAuthority: null,
                    },
                    mintContext: {
                        version: 1,
                        splMintInitialized: false,
                        splMint: PublicKey.default,
                    },
                    extensions: null,
                },
            },
            {
                description: 'mint with all authorities set',
                mint: {
                    base: {
                        mintAuthority: Keypair.generate().publicKey,
                        supply: BigInt(1_000_000_000),
                        decimals: 6,
                        isInitialized: true,
                        freezeAuthority: Keypair.generate().publicKey,
                    },
                    mintContext: {
                        version: 1,
                        splMintInitialized: true,
                        splMint: Keypair.generate().publicKey,
                    },
                    extensions: null,
                },
            },
            {
                description: 'mint with only mintAuthority',
                mint: {
                    base: {
                        mintAuthority: Keypair.generate().publicKey,
                        supply: BigInt(500),
                        decimals: 0,
                        isInitialized: true,
                        freezeAuthority: null,
                    },
                    mintContext: {
                        version: 0,
                        splMintInitialized: false,
                        splMint: PublicKey.default,
                    },
                    extensions: null,
                },
            },
            {
                description: 'mint with only freezeAuthority',
                mint: {
                    base: {
                        mintAuthority: null,
                        supply: BigInt('18446744073709551615'), // max u64
                        decimals: 18,
                        isInitialized: true,
                        freezeAuthority: Keypair.generate().publicKey,
                    },
                    mintContext: {
                        version: 255,
                        splMintInitialized: true,
                        splMint: Keypair.generate().publicKey,
                    },
                    extensions: null,
                },
            },
            {
                description: 'uninitialized mint',
                mint: {
                    base: {
                        mintAuthority: null,
                        supply: BigInt(0),
                        decimals: 0,
                        isInitialized: false,
                        freezeAuthority: null,
                    },
                    mintContext: {
                        version: 0,
                        splMintInitialized: false,
                        splMint: PublicKey.default,
                    },
                    extensions: null,
                },
            },
        ];

        testCases.forEach(({ description, mint }) => {
            it(`should roundtrip: ${description}`, () => {
                const serialized = serializeMint(mint);
                const deserialized = deserializeMint(serialized);

                // Compare base mint
                if (mint.base.mintAuthority) {
                    expect(deserialized.base.mintAuthority?.toBase58()).toBe(
                        mint.base.mintAuthority.toBase58(),
                    );
                } else {
                    expect(deserialized.base.mintAuthority).toBeNull();
                }

                expect(deserialized.base.supply).toBe(mint.base.supply);
                expect(deserialized.base.decimals).toBe(mint.base.decimals);
                expect(deserialized.base.isInitialized).toBe(
                    mint.base.isInitialized,
                );

                if (mint.base.freezeAuthority) {
                    expect(deserialized.base.freezeAuthority?.toBase58()).toBe(
                        mint.base.freezeAuthority.toBase58(),
                    );
                } else {
                    expect(deserialized.base.freezeAuthority).toBeNull();
                }

                // Compare mint context
                expect(deserialized.mintContext.version).toBe(
                    mint.mintContext.version,
                );
                expect(deserialized.mintContext.splMintInitialized).toBe(
                    mint.mintContext.splMintInitialized,
                );
                expect(deserialized.mintContext.splMint.toBase58()).toBe(
                    mint.mintContext.splMint.toBase58(),
                );

                // Compare extensions
                expect(deserialized.extensions).toEqual(mint.extensions);
            });
        });

        it('should produce expected buffer size for mint without extensions', () => {
            const mint: CompressedMint = {
                base: {
                    mintAuthority: null,
                    supply: BigInt(0),
                    decimals: 9,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    splMintInitialized: false,
                    splMint: PublicKey.default,
                },
                extensions: null,
            };

            const serialized = serializeMint(mint);
            // 82 (MINT_SIZE) + 34 (MINT_CONTEXT_SIZE) + 1 (None option byte)
            expect(serialized.length).toBe(MINT_SIZE + MINT_CONTEXT_SIZE + 1);
        });
    });

    describe('serializeMint with extensions', () => {
        it('should serialize mint with single extension (no length prefix - Borsh format)', () => {
            const extensionData = Buffer.from([1, 2, 3, 4, 5]);
            const mint: CompressedMint = {
                base: {
                    mintAuthority: null,
                    supply: BigInt(1000),
                    decimals: 9,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    splMintInitialized: false,
                    splMint: PublicKey.default,
                },
                extensions: [
                    {
                        extensionType: ExtensionType.TokenMetadata,
                        data: extensionData,
                    },
                ],
            };

            const serialized = serializeMint(mint);

            // Borsh format: Some(1) + vec_len(4) + discriminant(1) + data (NO length prefix)
            const expectedExtensionBytes = 1 + 4 + 1 + extensionData.length;
            expect(serialized.length).toBe(
                MINT_SIZE + MINT_CONTEXT_SIZE + expectedExtensionBytes,
            );
        });

        it('should serialize mint with multiple extensions (no length prefix)', () => {
            const ext1Data = Buffer.from([1, 2, 3]);
            const ext2Data = Buffer.from([4, 5, 6, 7, 8]);
            const mint: CompressedMint = {
                base: {
                    mintAuthority: null,
                    supply: BigInt(0),
                    decimals: 9,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    splMintInitialized: false,
                    splMint: PublicKey.default,
                },
                extensions: [
                    { extensionType: 1, data: ext1Data },
                    { extensionType: 2, data: ext2Data },
                ],
            };

            const serialized = serializeMint(mint);

            // Borsh format: Some(1) + vec_len(4) + (type(1) + data) for each (no length prefix)
            const expectedExtensionBytes = 1 + 4 + (1 + 3) + (1 + 5);
            expect(serialized.length).toBe(
                MINT_SIZE + MINT_CONTEXT_SIZE + expectedExtensionBytes,
            );
        });

        it('should serialize mint with empty extensions array as None', () => {
            const mint: CompressedMint = {
                base: {
                    mintAuthority: null,
                    supply: BigInt(0),
                    decimals: 9,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    splMintInitialized: false,
                    splMint: PublicKey.default,
                },
                extensions: [],
            };

            const serialized = serializeMint(mint);

            // Empty extensions array is treated as None (1 byte)
            expect(serialized.length).toBe(MINT_SIZE + MINT_CONTEXT_SIZE + 1);
            // The last byte should be 0 (None)
            expect(serialized[serialized.length - 1]).toBe(0);
        });
    });

    describe('decodeTokenMetadata / encodeTokenMetadata', () => {
        const testCases: { description: string; metadata: TokenMetadata }[] = [
            {
                description: 'basic metadata',
                metadata: {
                    mint: Keypair.generate().publicKey,
                    name: 'Test Token',
                    symbol: 'TEST',
                    uri: 'https://example.com/token.json',
                },
            },
            {
                description: 'metadata with updateAuthority',
                metadata: {
                    updateAuthority: Keypair.generate().publicKey,
                    mint: Keypair.generate().publicKey,
                    name: 'My Token',
                    symbol: 'MTK',
                    uri: 'ipfs://QmTest123',
                },
            },
            {
                description: 'metadata with additional metadata',
                metadata: {
                    mint: Keypair.generate().publicKey,
                    name: 'Rich Token',
                    symbol: 'RICH',
                    uri: 'https://arweave.net/xyz',
                    additionalMetadata: [
                        { key: 'description', value: 'A rich token' },
                        { key: 'image', value: 'https://example.com/img.png' },
                    ],
                },
            },
            {
                description: 'metadata with all fields',
                metadata: {
                    updateAuthority: Keypair.generate().publicKey,
                    mint: Keypair.generate().publicKey,
                    name: 'Full Token',
                    symbol: 'FULL',
                    uri: 'https://full.example.com/metadata.json',
                    additionalMetadata: [
                        { key: 'creator', value: 'Alice' },
                        { key: 'version', value: '1.0.0' },
                        { key: 'category', value: 'utility' },
                    ],
                },
            },
            {
                description: 'metadata with empty strings',
                metadata: {
                    mint: Keypair.generate().publicKey,
                    name: '',
                    symbol: '',
                    uri: '',
                },
            },
            {
                description: 'metadata with unicode characters',
                metadata: {
                    mint: Keypair.generate().publicKey,
                    name: 'Token',
                    symbol: 'TKN',
                    uri: 'https://example.com',
                },
            },
            {
                description: 'metadata with long values',
                metadata: {
                    mint: Keypair.generate().publicKey,
                    name: 'A'.repeat(100),
                    symbol: 'B'.repeat(10),
                    uri: 'https://example.com/' + 'c'.repeat(200),
                },
            },
        ];

        testCases.forEach(({ description, metadata }) => {
            it(`should roundtrip: ${description}`, () => {
                const encoded = encodeTokenMetadata(metadata);
                const decoded = decodeTokenMetadata(encoded);

                expect(decoded).not.toBeNull();
                expect(decoded!.name).toBe(metadata.name);
                expect(decoded!.symbol).toBe(metadata.symbol);
                expect(decoded!.uri).toBe(metadata.uri);

                if (metadata.updateAuthority) {
                    expect(decoded!.updateAuthority?.toBase58()).toBe(
                        metadata.updateAuthority.toBase58(),
                    );
                }

                if (
                    metadata.additionalMetadata &&
                    metadata.additionalMetadata.length > 0
                ) {
                    expect(decoded!.additionalMetadata).toHaveLength(
                        metadata.additionalMetadata.length,
                    );
                    metadata.additionalMetadata.forEach((item, idx) => {
                        expect(decoded!.additionalMetadata![idx].key).toBe(
                            item.key,
                        );
                        expect(decoded!.additionalMetadata![idx].value).toBe(
                            item.value,
                        );
                    });
                }
            });
        });

        it('should return null for invalid data (too short)', () => {
            const shortBuffer = Buffer.alloc(50); // Less than 80 byte minimum
            const result = decodeTokenMetadata(shortBuffer);
            expect(result).toBeNull();
        });

        it('should return null for empty data', () => {
            const emptyBuffer = Buffer.alloc(0);
            const result = decodeTokenMetadata(emptyBuffer);
            expect(result).toBeNull();
        });
    });

    describe('extractTokenMetadata', () => {
        it('should return null for null extensions', () => {
            const result = extractTokenMetadata(null);
            expect(result).toBeNull();
        });

        it('should return null for empty extensions array', () => {
            const result = extractTokenMetadata([]);
            expect(result).toBeNull();
        });

        it('should return null when TokenMetadata extension not found', () => {
            const extensions: MintExtension[] = [
                { extensionType: 1, data: Buffer.from([1, 2, 3]) },
                { extensionType: 2, data: Buffer.from([4, 5, 6]) },
            ];
            const result = extractTokenMetadata(extensions);
            expect(result).toBeNull();
        });

        it('should extract and parse TokenMetadata extension', () => {
            const metadata: TokenMetadata = {
                updateAuthority: Keypair.generate().publicKey,
                mint: Keypair.generate().publicKey,
                name: 'Extract Test',
                symbol: 'EXT',
                uri: 'https://extract.test',
            };

            const encodedMetadata = encodeTokenMetadata(metadata);
            const extensions: MintExtension[] = [
                {
                    extensionType: ExtensionType.TokenMetadata,
                    data: encodedMetadata,
                },
            ];

            const result = extractTokenMetadata(extensions);

            expect(result).not.toBeNull();
            expect(result!.name).toBe(metadata.name);
            expect(result!.symbol).toBe(metadata.symbol);
            expect(result!.uri).toBe(metadata.uri);
        });

        it('should find TokenMetadata among multiple extensions', () => {
            const metadata: TokenMetadata = {
                mint: Keypair.generate().publicKey,
                name: 'Multi Test',
                symbol: 'MLT',
                uri: 'https://multi.test',
            };

            const encodedMetadata = encodeTokenMetadata(metadata);
            const extensions: MintExtension[] = [
                { extensionType: 1, data: Buffer.from([1, 2, 3]) },
                {
                    extensionType: ExtensionType.TokenMetadata,
                    data: encodedMetadata,
                },
                { extensionType: 2, data: Buffer.from([4, 5, 6]) },
            ];

            const result = extractTokenMetadata(extensions);

            expect(result).not.toBeNull();
            expect(result!.name).toBe(metadata.name);
        });
    });

    describe('ExtensionType enum', () => {
        it('should have correct value for TokenMetadata', () => {
            expect(ExtensionType.TokenMetadata).toBe(19);
        });
    });

    describe('deserializeMint edge cases', () => {
        it('should handle Uint8Array input', () => {
            const mint: CompressedMint = {
                base: {
                    mintAuthority: null,
                    supply: BigInt(1000),
                    decimals: 9,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    splMintInitialized: false,
                    splMint: PublicKey.default,
                },
                extensions: null,
            };

            const serialized = serializeMint(mint);
            const uint8Array = new Uint8Array(serialized);
            const deserialized = deserializeMint(uint8Array);

            expect(deserialized.base.supply).toBe(mint.base.supply);
            expect(deserialized.base.decimals).toBe(mint.base.decimals);
        });

        it('should correctly parse version byte', () => {
            const testVersions = [0, 1, 127, 255];

            testVersions.forEach(version => {
                const mint: CompressedMint = {
                    base: {
                        mintAuthority: null,
                        supply: BigInt(0),
                        decimals: 9,
                        isInitialized: true,
                        freezeAuthority: null,
                    },
                    mintContext: {
                        version,
                        splMintInitialized: false,
                        splMint: PublicKey.default,
                    },
                    extensions: null,
                };

                const serialized = serializeMint(mint);
                const deserialized = deserializeMint(serialized);

                expect(deserialized.mintContext.version).toBe(version);
            });
        });

        it('should correctly parse splMintInitialized boolean', () => {
            [true, false].forEach(initialized => {
                const mint: CompressedMint = {
                    base: {
                        mintAuthority: null,
                        supply: BigInt(0),
                        decimals: 9,
                        isInitialized: true,
                        freezeAuthority: null,
                    },
                    mintContext: {
                        version: 1,
                        splMintInitialized: initialized,
                        splMint: PublicKey.default,
                    },
                    extensions: null,
                };

                const serialized = serializeMint(mint);
                const deserialized = deserializeMint(serialized);

                expect(deserialized.mintContext.splMintInitialized).toBe(
                    initialized,
                );
            });
        });
    });

    describe('serializeMint / deserializeMint specific pubkey values', () => {
        it('should handle specific well-known pubkeys', () => {
            const specificPubkeys = [
                PublicKey.default,
                new PublicKey('11111111111111111111111111111111'),
                new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
                new PublicKey('So11111111111111111111111111111111111111112'),
            ];

            specificPubkeys.forEach(pubkey => {
                const mint: CompressedMint = {
                    base: {
                        mintAuthority: pubkey,
                        supply: BigInt(0),
                        decimals: 9,
                        isInitialized: true,
                        freezeAuthority: pubkey,
                    },
                    mintContext: {
                        version: 1,
                        splMintInitialized: true,
                        splMint: pubkey,
                    },
                    extensions: null,
                };

                const serialized = serializeMint(mint);
                const deserialized = deserializeMint(serialized);

                expect(deserialized.base.mintAuthority?.toBase58()).toBe(
                    pubkey.toBase58(),
                );
                expect(deserialized.base.freezeAuthority?.toBase58()).toBe(
                    pubkey.toBase58(),
                );
                expect(deserialized.mintContext.splMint.toBase58()).toBe(
                    pubkey.toBase58(),
                );
            });
        });
    });

    describe('supply edge cases', () => {
        it('should handle zero supply', () => {
            const mint: CompressedMint = {
                base: {
                    mintAuthority: null,
                    supply: BigInt(0),
                    decimals: 9,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    splMintInitialized: false,
                    splMint: PublicKey.default,
                },
                extensions: null,
            };

            const serialized = serializeMint(mint);
            const deserialized = deserializeMint(serialized);

            expect(deserialized.base.supply).toBe(BigInt(0));
        });

        it('should handle large supply values', () => {
            const largeSupplies = [
                BigInt(1_000_000_000),
                BigInt('1000000000000000000'),
                BigInt('18446744073709551615'), // max u64
            ];

            largeSupplies.forEach(supply => {
                const mint: CompressedMint = {
                    base: {
                        mintAuthority: null,
                        supply,
                        decimals: 9,
                        isInitialized: true,
                        freezeAuthority: null,
                    },
                    mintContext: {
                        version: 1,
                        splMintInitialized: false,
                        splMint: PublicKey.default,
                    },
                    extensions: null,
                };

                const serialized = serializeMint(mint);
                const deserialized = deserializeMint(serialized);

                expect(deserialized.base.supply).toBe(supply);
            });
        });
    });

    describe('decimals edge cases', () => {
        it('should handle all valid decimal values (0-255)', () => {
            [0, 1, 6, 9, 18, 255].forEach(decimals => {
                const mint: CompressedMint = {
                    base: {
                        mintAuthority: null,
                        supply: BigInt(0),
                        decimals,
                        isInitialized: true,
                        freezeAuthority: null,
                    },
                    mintContext: {
                        version: 1,
                        splMintInitialized: false,
                        splMint: PublicKey.default,
                    },
                    extensions: null,
                };

                const serialized = serializeMint(mint);
                const deserialized = deserializeMint(serialized);

                expect(deserialized.base.decimals).toBe(decimals);
            });
        });
    });

    describe('deserializeMint with extensions', () => {
        it('should roundtrip serialize/deserialize with TokenMetadata extension', () => {
            const metadata: TokenMetadata = {
                updateAuthority: Keypair.generate().publicKey,
                mint: Keypair.generate().publicKey,
                name: 'Test Token',
                symbol: 'TEST',
                uri: 'https://example.com/metadata.json',
            };

            const encodedMetadata = encodeTokenMetadata(metadata);
            const mint: CompressedMint = {
                base: {
                    mintAuthority: Keypair.generate().publicKey,
                    supply: BigInt(1_000_000),
                    decimals: 9,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    splMintInitialized: true,
                    splMint: Keypair.generate().publicKey,
                },
                extensions: [
                    {
                        extensionType: ExtensionType.TokenMetadata,
                        data: encodedMetadata,
                    },
                ],
            };

            const serialized = serializeMint(mint);
            const deserialized = deserializeMint(serialized);

            // Base mint should roundtrip
            expect(deserialized.base.supply).toBe(mint.base.supply);
            expect(deserialized.base.decimals).toBe(mint.base.decimals);

            // Should have extensions
            expect(deserialized.extensions).not.toBeNull();
            expect(deserialized.extensions!.length).toBe(1);
            expect(deserialized.extensions![0].extensionType).toBe(
                ExtensionType.TokenMetadata,
            );

            // Extension data should be extractable and match original
            const extractedMetadata = extractTokenMetadata(
                deserialized.extensions,
            );
            expect(extractedMetadata).not.toBeNull();
            expect(extractedMetadata!.name).toBe(metadata.name);
            expect(extractedMetadata!.symbol).toBe(metadata.symbol);
            expect(extractedMetadata!.uri).toBe(metadata.uri);
        });

        it('should handle extension with hasExtensions=false', () => {
            const mint: CompressedMint = {
                base: {
                    mintAuthority: null,
                    supply: BigInt(0),
                    decimals: 9,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    splMintInitialized: false,
                    splMint: PublicKey.default,
                },
                extensions: null,
            };

            const serialized = serializeMint(mint);
            const deserialized = deserializeMint(serialized);

            expect(deserialized.extensions).toBeNull();
        });

        it('should correctly parse Borsh format (discriminant + data, no length prefix)', () => {
            const metadata: TokenMetadata = {
                updateAuthority: Keypair.generate().publicKey,
                mint: Keypair.generate().publicKey,
                name: 'Test Token',
                symbol: 'TEST',
                uri: 'https://example.com/metadata.json',
            };

            const encodedMetadata = encodeTokenMetadata(metadata);

            // Build buffer in Borsh format manually
            const baseMintBuffer = Buffer.alloc(MINT_SIZE);
            const contextBuffer = Buffer.alloc(MINT_CONTEXT_SIZE);

            // Borsh format: Some(1) + vec_len(4) + discriminant(1) + data (no length prefix)
            const extensionsBuffer = Buffer.concat([
                Buffer.from([1]), // Some
                Buffer.from([1, 0, 0, 0]), // vec len = 1
                Buffer.from([ExtensionType.TokenMetadata]), // discriminant
                encodedMetadata, // data directly (no length prefix)
            ]);

            const fullBuffer = Buffer.concat([
                baseMintBuffer,
                contextBuffer,
                extensionsBuffer,
            ]);

            const deserialized = deserializeMint(fullBuffer);

            expect(deserialized.extensions).not.toBeNull();
            expect(deserialized.extensions!.length).toBe(1);
            expect(deserialized.extensions![0].extensionType).toBe(
                ExtensionType.TokenMetadata,
            );

            // Metadata should be extractable
            const extractedMetadata = extractTokenMetadata(
                deserialized.extensions,
            );
            expect(extractedMetadata).not.toBeNull();
            expect(extractedMetadata!.name).toBe(metadata.name);
            expect(extractedMetadata!.symbol).toBe(metadata.symbol);
            expect(extractedMetadata!.uri).toBe(metadata.uri);
        });

        it('should handle multiple extensions', () => {
            const metadata1: TokenMetadata = {
                mint: Keypair.generate().publicKey,
                name: 'Token 1',
                symbol: 'T1',
                uri: 'https://example.com/1.json',
            };
            const metadata2: TokenMetadata = {
                mint: Keypair.generate().publicKey,
                name: 'Token 2',
                symbol: 'T2',
                uri: 'https://example.com/2.json',
            };

            const mint: CompressedMint = {
                base: {
                    mintAuthority: null,
                    supply: BigInt(0),
                    decimals: 9,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    splMintInitialized: false,
                    splMint: PublicKey.default,
                },
                extensions: [
                    {
                        extensionType: ExtensionType.TokenMetadata,
                        data: encodeTokenMetadata(metadata1),
                    },
                    {
                        extensionType: ExtensionType.TokenMetadata,
                        data: encodeTokenMetadata(metadata2),
                    },
                ],
            };

            const serialized = serializeMint(mint);
            const deserialized = deserializeMint(serialized);

            expect(deserialized.extensions).not.toBeNull();
            expect(deserialized.extensions!.length).toBe(2);

            const ext1Metadata = decodeTokenMetadata(
                deserialized.extensions![0].data,
            );
            const ext2Metadata = decodeTokenMetadata(
                deserialized.extensions![1].data,
            );

            expect(ext1Metadata!.name).toBe(metadata1.name);
            expect(ext2Metadata!.name).toBe(metadata2.name);
        });
    });

    describe('parseTokenMetadata alias', () => {
        it('parseTokenMetadata should be alias for decodeTokenMetadata', () => {
            // parseTokenMetadata is exported as an alias (deprecated)
            expect(parseTokenMetadata).toBe(decodeTokenMetadata);
        });
    });

    describe('TokenMetadata updateAuthority edge cases', () => {
        it('should return undefined for zero updateAuthority when decoding', () => {
            // Encode with no updateAuthority (uses zero pubkey)
            const metadata: TokenMetadata = {
                mint: Keypair.generate().publicKey,
                name: 'Test',
                symbol: 'TST',
                uri: 'https://test.com',
            };

            const encoded = encodeTokenMetadata(metadata);
            const decoded = decodeTokenMetadata(encoded);

            expect(decoded).not.toBeNull();
            // Zero pubkey should be returned as undefined
            expect(decoded!.updateAuthority).toBeUndefined();
        });

        it('should preserve non-zero updateAuthority', () => {
            const authority = Keypair.generate().publicKey;
            const metadata: TokenMetadata = {
                updateAuthority: authority,
                mint: Keypair.generate().publicKey,
                name: 'Test',
                symbol: 'TST',
                uri: 'https://test.com',
            };

            const encoded = encodeTokenMetadata(metadata);
            const decoded = decodeTokenMetadata(encoded);

            expect(decoded).not.toBeNull();
            expect(decoded!.updateAuthority).not.toBeUndefined();
            expect(decoded!.updateAuthority!.toBase58()).toBe(
                authority.toBase58(),
            );
        });

        it('should handle null updateAuthority same as undefined', () => {
            const metadata: TokenMetadata = {
                updateAuthority: null,
                mint: Keypair.generate().publicKey,
                name: 'Test',
                symbol: 'TST',
                uri: 'https://test.com',
            };

            const encoded = encodeTokenMetadata(metadata);
            const decoded = decodeTokenMetadata(encoded);

            expect(decoded).not.toBeNull();
            expect(decoded!.updateAuthority).toBeUndefined();
        });
    });

    describe('TokenMetadata with mint field (encoding includes mint)', () => {
        it('TokenMetadataLayout should include mint field in encoding', () => {
            // Verify the layout includes the mint field
            const mintPubkey = Keypair.generate().publicKey;
            const metadata: TokenMetadata = {
                mint: mintPubkey,
                name: 'Test',
                symbol: 'TST',
                uri: 'https://test.com',
            };

            const encoded = encodeTokenMetadata(metadata);

            // Encoded should have: updateAuthority (32) + mint (32) + name vec + symbol vec + uri vec + additional vec
            // Minimum: 32 + 32 + 4 + 4 + 4 + 4 = 80 bytes
            expect(encoded.length).toBeGreaterThanOrEqual(80);
        });

        it('encodeTokenMetadata should encode mint field correctly', () => {
            const mintPubkey = Keypair.generate().publicKey;
            const metadata: TokenMetadata = {
                mint: mintPubkey,
                name: 'Test',
                symbol: 'TST',
                uri: 'https://test.com',
            };

            const encoded = encodeTokenMetadata(metadata);

            // Bytes 32-63 should be the mint pubkey (after updateAuthority)
            const mintBytes = encoded.slice(32, 64);
            expect(Buffer.from(mintBytes).equals(mintPubkey.toBuffer())).toBe(
                true,
            );
        });
    });

    describe('decodeTokenMetadata malformed data', () => {
        it('should return null for data shorter than 80 bytes (minimum Borsh size)', () => {
            const shortData = Buffer.alloc(79);
            expect(decodeTokenMetadata(shortData)).toBeNull();
        });

        it('should decode 80 bytes of zeros as empty metadata', () => {
            // Minimum size: 32 (updateAuthority) + 32 (mint) + 4*4 (vec lengths) = 80 bytes
            // All zeros means: zero pubkeys and empty vecs - this is actually valid
            const data = Buffer.alloc(80);
            const result = decodeTokenMetadata(data);
            expect(result).not.toBeNull();
            expect(result!.name).toBe('');
            expect(result!.symbol).toBe('');
            expect(result!.uri).toBe('');
            expect(result!.updateAuthority).toBeUndefined(); // zero pubkey -> undefined
        });

        it('should handle corrupted vec length gracefully', () => {
            // Create valid header but corrupted name length
            const data = Buffer.alloc(100);
            // Set name length to a huge value at offset 64 (after updateAuthority + mint)
            data.writeUInt32LE(0xffffffff, 64);
            // Should return null due to try/catch
            expect(decodeTokenMetadata(data)).toBeNull();
        });
    });

    describe('encodeTokenMetadata buffer allocation', () => {
        it('should handle metadata that fits within 2000 byte buffer', () => {
            const metadata: TokenMetadata = {
                mint: Keypair.generate().publicKey,
                name: 'A'.repeat(500),
                symbol: 'B'.repeat(100),
                uri: 'C'.repeat(500),
                additionalMetadata: [
                    { key: 'k1', value: 'v'.repeat(100) },
                    { key: 'k2', value: 'v'.repeat(100) },
                ],
            };

            const encoded = encodeTokenMetadata(metadata);
            expect(encoded.length).toBeLessThan(2000);

            // Should roundtrip
            const decoded = decodeTokenMetadata(encoded);
            expect(decoded!.name).toBe(metadata.name);
            expect(decoded!.symbol).toBe(metadata.symbol);
            expect(decoded!.uri).toBe(metadata.uri);
        });
    });

    describe('toMintInstructionData conversion', () => {
        it('should convert CompressedMint without extensions', () => {
            const splMint = Keypair.generate().publicKey;
            const mintAuthority = Keypair.generate().publicKey;

            const compressedMint: CompressedMint = {
                base: {
                    mintAuthority,
                    supply: BigInt(1_000_000),
                    decimals: 9,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    splMintInitialized: true,
                    splMint,
                },
                extensions: null,
            };

            const result = toMintInstructionData(compressedMint);

            expect(result.supply).toBe(BigInt(1_000_000));
            expect(result.decimals).toBe(9);
            expect(result.mintAuthority?.toBase58()).toBe(
                mintAuthority.toBase58(),
            );
            expect(result.freezeAuthority).toBeNull();
            expect(result.splMint.toBase58()).toBe(splMint.toBase58());
            expect(result.splMintInitialized).toBe(true);
            expect(result.version).toBe(1);
            expect(result.metadata).toBeUndefined();
        });

        it('should convert CompressedMint with TokenMetadata extension', () => {
            const splMint = Keypair.generate().publicKey;
            const updateAuthority = Keypair.generate().publicKey;

            const tokenMetadata: TokenMetadata = {
                updateAuthority,
                mint: Keypair.generate().publicKey,
                name: 'Test Token',
                symbol: 'TEST',
                uri: 'https://example.com/metadata.json',
            };

            const compressedMint: CompressedMint = {
                base: {
                    mintAuthority: null,
                    supply: BigInt(500_000),
                    decimals: 6,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 2,
                    splMintInitialized: false,
                    splMint,
                },
                extensions: [
                    {
                        extensionType: ExtensionType.TokenMetadata,
                        data: encodeTokenMetadata(tokenMetadata),
                    },
                ],
            };

            const result = toMintInstructionData(compressedMint);

            expect(result.supply).toBe(BigInt(500_000));
            expect(result.decimals).toBe(6);
            expect(result.version).toBe(2);
            expect(result.metadata).toBeDefined();
            expect(result.metadata!.name).toBe('Test Token');
            expect(result.metadata!.symbol).toBe('TEST');
            expect(result.metadata!.uri).toBe(
                'https://example.com/metadata.json',
            );
            expect(result.metadata!.updateAuthority?.toBase58()).toBe(
                updateAuthority.toBase58(),
            );
        });

        it('should handle CompressedMint with empty extensions array', () => {
            const compressedMint: CompressedMint = {
                base: {
                    mintAuthority: null,
                    supply: BigInt(0),
                    decimals: 9,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    splMintInitialized: false,
                    splMint: PublicKey.default,
                },
                extensions: [],
            };

            const result = toMintInstructionData(compressedMint);
            expect(result.metadata).toBeUndefined();
        });

        it('should handle metadata with null updateAuthority', () => {
            const tokenMetadata: TokenMetadata = {
                mint: Keypair.generate().publicKey,
                name: 'No Authority',
                symbol: 'NA',
                uri: 'https://example.com',
            };

            const compressedMint: CompressedMint = {
                base: {
                    mintAuthority: null,
                    supply: BigInt(100),
                    decimals: 0,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    splMintInitialized: false,
                    splMint: PublicKey.default,
                },
                extensions: [
                    {
                        extensionType: ExtensionType.TokenMetadata,
                        data: encodeTokenMetadata(tokenMetadata),
                    },
                ],
            };

            const result = toMintInstructionData(compressedMint);
            expect(result.metadata).toBeDefined();
            expect(result.metadata!.updateAuthority).toBeNull();
        });
    });

    describe('toMintInstructionDataWithMetadata conversion', () => {
        it('should convert CompressedMint with metadata extension', () => {
            const tokenMetadata: TokenMetadata = {
                updateAuthority: Keypair.generate().publicKey,
                mint: Keypair.generate().publicKey,
                name: 'With Metadata',
                symbol: 'WM',
                uri: 'https://wm.com',
            };

            const compressedMint: CompressedMint = {
                base: {
                    mintAuthority: Keypair.generate().publicKey,
                    supply: BigInt(1000),
                    decimals: 9,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    splMintInitialized: true,
                    splMint: Keypair.generate().publicKey,
                },
                extensions: [
                    {
                        extensionType: ExtensionType.TokenMetadata,
                        data: encodeTokenMetadata(tokenMetadata),
                    },
                ],
            };

            const result = toMintInstructionDataWithMetadata(compressedMint);

            // Metadata field should be required (not optional)
            expect(result.metadata.name).toBe('With Metadata');
            expect(result.metadata.symbol).toBe('WM');
            expect(result.metadata.uri).toBe('https://wm.com');
        });

        it('should throw if CompressedMint has no metadata extension', () => {
            const compressedMint: CompressedMint = {
                base: {
                    mintAuthority: null,
                    supply: BigInt(0),
                    decimals: 9,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    splMintInitialized: false,
                    splMint: PublicKey.default,
                },
                extensions: null,
            };

            expect(() =>
                toMintInstructionDataWithMetadata(compressedMint),
            ).toThrow('CompressedMint does not have TokenMetadata extension');
        });

        it('should throw if extensions array is empty', () => {
            const compressedMint: CompressedMint = {
                base: {
                    mintAuthority: null,
                    supply: BigInt(0),
                    decimals: 9,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    splMintInitialized: false,
                    splMint: PublicKey.default,
                },
                extensions: [],
            };

            expect(() =>
                toMintInstructionDataWithMetadata(compressedMint),
            ).toThrow('CompressedMint does not have TokenMetadata extension');
        });
    });

    describe('MintInstructionData type structure', () => {
        it('MintInstructionData should have correct shape', () => {
            const data: MintInstructionData = {
                supply: BigInt(1000),
                decimals: 9,
                mintAuthority: null,
                freezeAuthority: null,
                splMint: PublicKey.default,
                splMintInitialized: false,
                version: 1,
            };

            expect(data.supply).toBe(BigInt(1000));
            expect(data.decimals).toBe(9);
            expect(data.metadata).toBeUndefined();
        });

        it('MintInstructionDataWithMetadata should require metadata', () => {
            const data: MintInstructionDataWithMetadata = {
                supply: BigInt(1000),
                decimals: 9,
                mintAuthority: null,
                freezeAuthority: null,
                splMint: PublicKey.default,
                splMintInitialized: false,
                version: 1,
                metadata: {
                    updateAuthority: null,
                    name: 'Test',
                    symbol: 'T',
                    uri: 'https://test.com',
                },
            };

            expect(data.metadata.name).toBe('Test');
        });

        it('MintMetadataField should have correct shape', () => {
            const metadata: MintMetadataField = {
                updateAuthority: Keypair.generate().publicKey,
                name: 'Token Name',
                symbol: 'TN',
                uri: 'https://example.com',
            };

            expect(metadata.name).toBe('Token Name');
            expect(metadata.symbol).toBe('TN');
            expect(metadata.uri).toBe('https://example.com');
        });
    });
});
