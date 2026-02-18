import { describe, it, expect } from 'vitest';
import { PublicKey, Keypair } from '@solana/web3.js';
import {
    deserializeMint,
    serializeMint,
    decodeTokenMetadata,
    encodeTokenMetadata,
    extractTokenMetadata,
    toMintInstructionData,
    toMintInstructionDataWithMetadata,
    CompressedMint,
    BaseMint,
    MintContext,
    MintExtension,
    TokenMetadata,
    ExtensionType,
} from '../../src/v3/layout/layout-mint';

describe('layout-mint', () => {
    describe('serializeMint / deserializeMint', () => {
        it('should serialize and deserialize a basic mint without extensions', () => {
            const mintAuthority = Keypair.generate().publicKey;
            const splMint = Keypair.generate().publicKey;

            const mint: CompressedMint = {
                base: {
                    mintAuthority,
                    supply: 1000000n,
                    decimals: 9,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    cmintDecompressed: true,
                    splMint,
                    mintSigner: new Uint8Array(32),
                    bump: 254,
                },
                extensions: null,
            };

            const serialized = serializeMint(mint);
            const deserialized = deserializeMint(serialized);

            expect(deserialized.base.mintAuthority?.toBase58()).toBe(
                mintAuthority.toBase58(),
            );
            expect(deserialized.base.supply).toBe(1000000n);
            expect(deserialized.base.decimals).toBe(9);
            expect(deserialized.base.isInitialized).toBe(true);
            expect(deserialized.base.freezeAuthority).toBe(null);
            expect(deserialized.mintContext.version).toBe(1);
            expect(deserialized.mintContext.cmintDecompressed).toBe(true);
            expect(deserialized.mintContext.splMint.toBase58()).toBe(
                splMint.toBase58(),
            );
            expect(deserialized.extensions).toBe(null);
        });

        it('should serialize and deserialize a mint with freeze authority', () => {
            const mintAuthority = Keypair.generate().publicKey;
            const freezeAuthority = Keypair.generate().publicKey;
            const splMint = Keypair.generate().publicKey;

            const mint: CompressedMint = {
                base: {
                    mintAuthority,
                    supply: 500n,
                    decimals: 6,
                    isInitialized: true,
                    freezeAuthority,
                },
                mintContext: {
                    version: 0,
                    cmintDecompressed: false,
                    splMint,
                    mintSigner: new Uint8Array(32),
                    bump: 254,
                },
                extensions: null,
            };

            const serialized = serializeMint(mint);
            const deserialized = deserializeMint(serialized);

            expect(deserialized.base.freezeAuthority?.toBase58()).toBe(
                freezeAuthority.toBase58(),
            );
            expect(deserialized.mintContext.cmintDecompressed).toBe(false);
        });

        it('should handle null mintAuthority', () => {
            const splMint = Keypair.generate().publicKey;

            const mint: CompressedMint = {
                base: {
                    mintAuthority: null,
                    supply: 0n,
                    decimals: 0,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    cmintDecompressed: true,
                    splMint,
                    mintSigner: new Uint8Array(32),
                    bump: 254,
                },
                extensions: null,
            };

            const serialized = serializeMint(mint);
            const deserialized = deserializeMint(serialized);

            expect(deserialized.base.mintAuthority).toBe(null);
        });

        it('should serialize and deserialize a mint with token metadata extension', () => {
            const mintAuthority = Keypair.generate().publicKey;
            const splMint = Keypair.generate().publicKey;
            const updateAuthority = Keypair.generate().publicKey;

            // Create metadata
            const metadata: TokenMetadata = {
                updateAuthority,
                mint: splMint,
                name: 'Test Token',
                symbol: 'TEST',
                uri: 'https://example.com/metadata.json',
                additionalMetadata: [{ key: 'version', value: '1.0' }],
            };

            const encodedMetadata = encodeTokenMetadata(metadata);

            const mint: CompressedMint = {
                base: {
                    mintAuthority,
                    supply: 1000n,
                    decimals: 9,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    cmintDecompressed: true,
                    splMint,
                    mintSigner: new Uint8Array(32),
                    bump: 254,
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

            expect(deserialized.extensions).not.toBe(null);
            expect(deserialized.extensions?.length).toBe(1);
            expect(deserialized.extensions?.[0].extensionType).toBe(
                ExtensionType.TokenMetadata,
            );
        });

        it('should handle large supply values', () => {
            const splMint = Keypair.generate().publicKey;
            const largeSupply = BigInt('18446744073709551615'); // max u64

            const mint: CompressedMint = {
                base: {
                    mintAuthority: null,
                    supply: largeSupply,
                    decimals: 9,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    cmintDecompressed: true,
                    splMint,
                    mintSigner: new Uint8Array(32),
                    bump: 254,
                },
                extensions: null,
            };

            const serialized = serializeMint(mint);
            const deserialized = deserializeMint(serialized);

            expect(deserialized.base.supply).toBe(largeSupply);
        });
    });

    describe('encodeTokenMetadata / decodeTokenMetadata', () => {
        it('should encode and decode token metadata', () => {
            const mintPubkey = Keypair.generate().publicKey;
            const updateAuthority = Keypair.generate().publicKey;

            const metadata: TokenMetadata = {
                updateAuthority,
                mint: mintPubkey,
                name: 'My Token',
                symbol: 'MTK',
                uri: 'https://my-token.com/metadata.json',
            };

            const encoded = encodeTokenMetadata(metadata);
            const decoded = decodeTokenMetadata(encoded);

            expect(decoded).not.toBe(null);
            expect(decoded?.name).toBe('My Token');
            expect(decoded?.symbol).toBe('MTK');
            expect(decoded?.uri).toBe('https://my-token.com/metadata.json');
            expect(decoded?.mint.toBase58()).toBe(mintPubkey.toBase58());
            expect(decoded?.updateAuthority?.toBase58()).toBe(
                updateAuthority.toBase58(),
            );
        });

        it('should handle metadata with additional metadata fields', () => {
            const mintPubkey = Keypair.generate().publicKey;
            const updateAuthority = Keypair.generate().publicKey;

            const metadata: TokenMetadata = {
                updateAuthority,
                mint: mintPubkey,
                name: 'Extended Token',
                symbol: 'EXT',
                uri: 'https://example.com/extended.json',
                additionalMetadata: [
                    { key: 'version', value: '2.0' },
                    { key: 'creator', value: 'Light Protocol' },
                    { key: 'category', value: 'compressed' },
                ],
            };

            const encoded = encodeTokenMetadata(metadata);
            const decoded = decodeTokenMetadata(encoded);

            expect(decoded?.additionalMetadata?.length).toBe(3);
            expect(decoded?.additionalMetadata?.[0]).toEqual({
                key: 'version',
                value: '2.0',
            });
            expect(decoded?.additionalMetadata?.[1]).toEqual({
                key: 'creator',
                value: 'Light Protocol',
            });
        });

        it('should handle null updateAuthority (zero pubkey)', () => {
            const mintPubkey = Keypair.generate().publicKey;

            const metadata: TokenMetadata = {
                updateAuthority: null,
                mint: mintPubkey,
                name: 'Immutable Token',
                symbol: 'IMM',
                uri: 'https://example.com/immutable.json',
            };

            const encoded = encodeTokenMetadata(metadata);
            const decoded = decodeTokenMetadata(encoded);

            expect(decoded?.updateAuthority).toBeUndefined();
        });

        it('should handle empty strings', () => {
            const mintPubkey = Keypair.generate().publicKey;

            const metadata: TokenMetadata = {
                mint: mintPubkey,
                name: '',
                symbol: '',
                uri: '',
            };

            const encoded = encodeTokenMetadata(metadata);
            const decoded = decodeTokenMetadata(encoded);

            expect(decoded?.name).toBe('');
            expect(decoded?.symbol).toBe('');
            expect(decoded?.uri).toBe('');
        });

        it('should handle unicode characters', () => {
            const mintPubkey = Keypair.generate().publicKey;

            const metadata: TokenMetadata = {
                mint: mintPubkey,
                name: 'Token',
                symbol: '$TOKEN',
                uri: 'https://example.com/metadata.json',
            };

            const encoded = encodeTokenMetadata(metadata);
            const decoded = decodeTokenMetadata(encoded);

            expect(decoded?.name).toBe('Token');
            expect(decoded?.symbol).toBe('$TOKEN');
        });

        it('should return null for invalid data', () => {
            const invalidBuffer = Buffer.alloc(10); // Too small
            const decoded = decodeTokenMetadata(invalidBuffer);
            expect(decoded).toBe(null);
        });
    });

    describe('extractTokenMetadata', () => {
        it('should extract token metadata from extensions array', () => {
            const mintPubkey = Keypair.generate().publicKey;
            const metadata: TokenMetadata = {
                mint: mintPubkey,
                name: 'Test',
                symbol: 'TST',
                uri: 'https://test.com',
            };

            const encoded = encodeTokenMetadata(metadata);
            const extensions: MintExtension[] = [
                { extensionType: ExtensionType.TokenMetadata, data: encoded },
            ];

            const extracted = extractTokenMetadata(extensions);

            expect(extracted).not.toBe(null);
            expect(extracted?.name).toBe('Test');
        });

        it('should return null for null extensions', () => {
            const extracted = extractTokenMetadata(null);
            expect(extracted).toBe(null);
        });

        it('should return null when no metadata extension exists', () => {
            const extensions: MintExtension[] = [
                { extensionType: 99, data: Buffer.alloc(10) }, // Unknown extension
            ];

            const extracted = extractTokenMetadata(extensions);
            expect(extracted).toBe(null);
        });
    });

    describe('toMintInstructionData', () => {
        it('should convert CompressedMint to MintInstructionData', () => {
            const mintAuthority = Keypair.generate().publicKey;
            const splMint = Keypair.generate().publicKey;

            const compressedMint: CompressedMint = {
                base: {
                    mintAuthority,
                    supply: 5000n,
                    decimals: 6,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    cmintDecompressed: true,
                    splMint,
                    mintSigner: new Uint8Array(32),
                    bump: 254,
                },
                extensions: null,
            };

            const instructionData = toMintInstructionData(compressedMint);

            expect(instructionData.supply).toBe(5000n);
            expect(instructionData.decimals).toBe(6);
            expect(instructionData.mintAuthority?.toBase58()).toBe(
                mintAuthority.toBase58(),
            );
            expect(instructionData.freezeAuthority).toBe(null);
            expect(instructionData.splMint.toBase58()).toBe(splMint.toBase58());
            expect(instructionData.cmintDecompressed).toBe(true);
            expect(instructionData.version).toBe(1);
            expect(instructionData.metadata).toBeUndefined();
        });

        it('should include metadata when extension exists', () => {
            const mintAuthority = Keypair.generate().publicKey;
            const splMint = Keypair.generate().publicKey;
            const updateAuthority = Keypair.generate().publicKey;

            const metadata: TokenMetadata = {
                updateAuthority,
                mint: splMint,
                name: 'Instruction Token',
                symbol: 'INST',
                uri: 'https://inst.com/meta.json',
            };

            const compressedMint: CompressedMint = {
                base: {
                    mintAuthority,
                    supply: 1000n,
                    decimals: 9,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    cmintDecompressed: true,
                    splMint,
                    mintSigner: new Uint8Array(32),
                    bump: 254,
                },
                extensions: [
                    {
                        extensionType: ExtensionType.TokenMetadata,
                        data: encodeTokenMetadata(metadata),
                    },
                ],
            };

            const instructionData = toMintInstructionData(compressedMint);

            expect(instructionData.metadata).toBeDefined();
            expect(instructionData.metadata?.name).toBe('Instruction Token');
            expect(instructionData.metadata?.symbol).toBe('INST');
            expect(instructionData.metadata?.uri).toBe(
                'https://inst.com/meta.json',
            );
        });
    });

    describe('toMintInstructionDataWithMetadata', () => {
        it('should return data with required metadata', () => {
            const mintAuthority = Keypair.generate().publicKey;
            const splMint = Keypair.generate().publicKey;
            const updateAuthority = Keypair.generate().publicKey;

            const metadata: TokenMetadata = {
                updateAuthority,
                mint: splMint,
                name: 'Required Meta',
                symbol: 'REQ',
                uri: 'https://req.com/meta.json',
            };

            const compressedMint: CompressedMint = {
                base: {
                    mintAuthority,
                    supply: 1000n,
                    decimals: 9,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    cmintDecompressed: true,
                    splMint,
                    mintSigner: new Uint8Array(32),
                    bump: 254,
                },
                extensions: [
                    {
                        extensionType: ExtensionType.TokenMetadata,
                        data: encodeTokenMetadata(metadata),
                    },
                ],
            };

            const instructionData =
                toMintInstructionDataWithMetadata(compressedMint);

            expect(instructionData.metadata.name).toBe('Required Meta');
            expect(instructionData.metadata.symbol).toBe('REQ');
        });

        it('should throw when metadata extension is missing', () => {
            const mintAuthority = Keypair.generate().publicKey;
            const splMint = Keypair.generate().publicKey;

            const compressedMint: CompressedMint = {
                base: {
                    mintAuthority,
                    supply: 1000n,
                    decimals: 9,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    cmintDecompressed: true,
                    splMint,
                    mintSigner: new Uint8Array(32),
                    bump: 254,
                },
                extensions: null,
            };

            expect(() =>
                toMintInstructionDataWithMetadata(compressedMint),
            ).toThrow('light mint does not have TokenMetadata extension');
        });
    });
});
