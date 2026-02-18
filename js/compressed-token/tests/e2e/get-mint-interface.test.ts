import { describe, it, expect, beforeAll } from 'vitest';
import { Keypair, Signer, PublicKey, AccountInfo } from '@solana/web3.js';
import {
    Rpc,
    newAccountWithLamports,
    createRpc,
    VERSION,
    featureFlags,
    LIGHT_TOKEN_PROGRAM_ID,
} from '@lightprotocol/stateless.js';
import {
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    MintLayout,
    MINT_SIZE,
    createInitializeMintInstruction,
    createMint as createSplMint,
} from '@solana/spl-token';
import { Buffer } from 'buffer';
import {
    getMintInterface,
    unpackMintInterface,
    unpackMintData,
    MintInterface,
} from '../../src/v3/get-mint-interface';
import { createMintInterface } from '../../src/v3/actions';
import { createTokenMetadata } from '../../src/v3/instructions';
import { findMintAddress } from '../../src/v3/derivation';
import {
    serializeMint,
    encodeTokenMetadata,
    CompressedMint,
    MintContext,
    TokenMetadata,
    ExtensionType,
    MINT_CONTEXT_SIZE,
} from '../../src/v3/layout/layout-mint';

featureFlags.version = VERSION.V2;

describe('getMintInterface', () => {
    let rpc: Rpc;
    let payer: Signer;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
    });

    describe('CToken mint (LIGHT_TOKEN_PROGRAM_ID)', () => {
        it('should fetch compressed mint with explicit programId', async () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();
            const decimals = 9;
            const [mintPda] = findMintAddress(mintSigner.publicKey);

            await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                null,
                decimals,
                mintSigner,
                { skipPreflight: true },
            );

            const result = await getMintInterface(
                rpc,
                mintPda,
                undefined,
                LIGHT_TOKEN_PROGRAM_ID,
            );

            expect(result.mint.address.toBase58()).toBe(mintPda.toBase58());
            expect(result.mint.mintAuthority?.toBase58()).toBe(
                mintAuthority.publicKey.toBase58(),
            );
            expect(result.mint.decimals).toBe(decimals);
            expect(result.mint.supply).toBe(0n);
            expect(result.mint.isInitialized).toBe(true);
            expect(result.mint.freezeAuthority).toBeNull();
            expect(result.programId.toBase58()).toBe(
                LIGHT_TOKEN_PROGRAM_ID.toBase58(),
            );
            expect(result.merkleContext).toBeDefined();
            expect(result.mintContext).toBeDefined();
        });

        it('should fetch compressed mint with freeze authority', async () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();
            const freezeAuthority = Keypair.generate();
            const decimals = 6;
            const [mintPda] = findMintAddress(mintSigner.publicKey);

            await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                freezeAuthority.publicKey,
                decimals,
                mintSigner,
                { skipPreflight: true },
            );

            const result = await getMintInterface(
                rpc,
                mintPda,
                undefined,
                LIGHT_TOKEN_PROGRAM_ID,
            );

            expect(result.mint.freezeAuthority?.toBase58()).toBe(
                freezeAuthority.publicKey.toBase58(),
            );
        });

        it('should fetch compressed mint with metadata', async () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();
            const decimals = 9;
            const [mintPda] = findMintAddress(mintSigner.publicKey);

            const metadata = createTokenMetadata(
                'Test Token',
                'TEST',
                'https://example.com/metadata.json',
                mintAuthority.publicKey,
            );

            await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                null,
                decimals,
                mintSigner,
                { skipPreflight: true },
                LIGHT_TOKEN_PROGRAM_ID,
                metadata,
            );

            const result = await getMintInterface(
                rpc,
                mintPda,
                undefined,
                LIGHT_TOKEN_PROGRAM_ID,
            );

            expect(result.tokenMetadata).toBeDefined();
            expect(result.tokenMetadata!.name).toBe('Test Token');
            expect(result.tokenMetadata!.symbol).toBe('TEST');
            expect(result.tokenMetadata!.uri).toBe(
                'https://example.com/metadata.json',
            );
            expect(result.extensions).toBeDefined();
            expect(result.extensions!.length).toBeGreaterThan(0);
        });

        it('should throw for non-existent compressed mint', async () => {
            const fakeMint = Keypair.generate().publicKey;

            await expect(
                getMintInterface(
                    rpc,
                    fakeMint,
                    undefined,
                    LIGHT_TOKEN_PROGRAM_ID,
                ),
            ).rejects.toThrow('Compressed mint not found');
        });
    });

    describe('SPL Token mint (TOKEN_PROGRAM_ID)', () => {
        it('should fetch SPL mint with explicit programId', async () => {
            const mintAuthority = Keypair.generate();
            const decimals = 9;

            const mint = await createSplMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                null,
                decimals,
                undefined,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            const result = await getMintInterface(
                rpc,
                mint,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            expect(result.mint.address.toBase58()).toBe(mint.toBase58());
            expect(result.mint.mintAuthority?.toBase58()).toBe(
                mintAuthority.publicKey.toBase58(),
            );
            expect(result.mint.decimals).toBe(decimals);
            expect(result.mint.supply).toBe(0n);
            expect(result.mint.isInitialized).toBe(true);
            expect(result.programId.toBase58()).toBe(
                TOKEN_PROGRAM_ID.toBase58(),
            );
            expect(result.merkleContext).toBeUndefined();
            expect(result.mintContext).toBeUndefined();
            expect(result.tokenMetadata).toBeUndefined();
        });

        it('should fetch SPL mint with freeze authority', async () => {
            const mintAuthority = Keypair.generate();
            const freezeAuthority = Keypair.generate();
            const decimals = 6;

            const mint = await createSplMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                freezeAuthority.publicKey,
                decimals,
                undefined,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            const result = await getMintInterface(
                rpc,
                mint,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            expect(result.mint.freezeAuthority?.toBase58()).toBe(
                freezeAuthority.publicKey.toBase58(),
            );
        });
    });

    describe('Token-2022 mint (TOKEN_2022_PROGRAM_ID)', () => {
        it('should fetch Token-2022 mint with explicit programId', async () => {
            const mintAuthority = Keypair.generate();
            const decimals = 9;

            const mint = await createSplMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                null,
                decimals,
                undefined,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );

            const result = await getMintInterface(
                rpc,
                mint,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );

            expect(result.mint.address.toBase58()).toBe(mint.toBase58());
            expect(result.mint.mintAuthority?.toBase58()).toBe(
                mintAuthority.publicKey.toBase58(),
            );
            expect(result.mint.decimals).toBe(decimals);
            expect(result.programId.toBase58()).toBe(
                TOKEN_2022_PROGRAM_ID.toBase58(),
            );
            expect(result.merkleContext).toBeUndefined();
            expect(result.mintContext).toBeUndefined();
        });
    });

    describe('Auto-detect mode (no programId)', () => {
        it('should auto-detect SPL mint', async () => {
            const mintAuthority = Keypair.generate();
            const decimals = 9;

            const mint = await createSplMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                null,
                decimals,
                undefined,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            const result = await getMintInterface(rpc, mint);

            expect(result.mint.address.toBase58()).toBe(mint.toBase58());
            expect(result.programId.toBase58()).toBe(
                TOKEN_PROGRAM_ID.toBase58(),
            );
        });

        it('should auto-detect Token-2022 mint', async () => {
            const mintAuthority = Keypair.generate();
            const decimals = 6;

            const mint = await createSplMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                null,
                decimals,
                undefined,
                undefined,
                TOKEN_2022_PROGRAM_ID,
            );

            const result = await getMintInterface(rpc, mint);

            expect(result.mint.address.toBase58()).toBe(mint.toBase58());
            // Could be detected as either T22 or SPL depending on priority
            expect([
                TOKEN_PROGRAM_ID.toBase58(),
                TOKEN_2022_PROGRAM_ID.toBase58(),
            ]).toContain(result.programId.toBase58());
        });

        it('should auto-detect compressed mint', async () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();
            const decimals = 9;
            const [mintPda] = findMintAddress(mintSigner.publicKey);

            await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                null,
                decimals,
                mintSigner,
                { skipPreflight: true },
            );

            const result = await getMintInterface(rpc, mintPda);

            expect(result.mint.address.toBase58()).toBe(mintPda.toBase58());
            expect(result.programId.toBase58()).toBe(
                LIGHT_TOKEN_PROGRAM_ID.toBase58(),
            );
            expect(result.merkleContext).toBeDefined();
            expect(result.mintContext).toBeDefined();
        });

        it('should throw for non-existent mint in auto-detect mode', async () => {
            const fakeMint = Keypair.generate().publicKey;

            await expect(getMintInterface(rpc, fakeMint)).rejects.toThrow(
                'Mint not found',
            );
        });
    });

    describe('mintContext validation', () => {
        it('should have valid mintContext for compressed mint', async () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();
            const [mintPda] = findMintAddress(mintSigner.publicKey);

            await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                null,
                9,
                mintSigner,
                { skipPreflight: true },
            );

            const result = await getMintInterface(
                rpc,
                mintPda,
                undefined,
                LIGHT_TOKEN_PROGRAM_ID,
            );

            expect(result.mintContext).toBeDefined();
            expect(result.mintContext!.version).toBeDefined();
            expect(typeof result.mintContext!.cmintDecompressed).toBe(
                'boolean',
            );
            expect(result.mintContext!.splMint).toBeInstanceOf(PublicKey);
        });
    });

    describe('merkleContext validation', () => {
        it('should have valid merkleContext for compressed mint', async () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();
            const [mintPda] = findMintAddress(mintSigner.publicKey);

            await createMintInterface(
                rpc,
                payer,
                mintAuthority,
                null,
                9,
                mintSigner,
                { skipPreflight: true },
            );

            const result = await getMintInterface(
                rpc,
                mintPda,
                undefined,
                LIGHT_TOKEN_PROGRAM_ID,
            );

            expect(result.merkleContext).toBeDefined();
            expect(result.merkleContext!.treeInfo).toBeDefined();
            expect(result.merkleContext!.hash).toBeDefined();
            expect(result.merkleContext!.leafIndex).toBeDefined();
        });
    });
});

describe('unpackMintInterface', () => {
    describe('SPL Token mint', () => {
        it('should unpack SPL mint data', () => {
            const mintAddress = Keypair.generate().publicKey;
            const mintAuthority = Keypair.generate().publicKey;
            const freezeAuthority = Keypair.generate().publicKey;

            const buffer = Buffer.alloc(MINT_SIZE);
            MintLayout.encode(
                {
                    mintAuthorityOption: 1,
                    mintAuthority,
                    supply: BigInt(1_000_000),
                    decimals: 9,
                    isInitialized: true,
                    freezeAuthorityOption: 1,
                    freezeAuthority,
                },
                buffer,
            );

            const accountInfo: AccountInfo<Buffer> = {
                data: buffer,
                executable: false,
                lamports: 1_000_000,
                owner: TOKEN_PROGRAM_ID,
                rentEpoch: 0,
            };

            const result = unpackMintInterface(
                mintAddress,
                accountInfo,
                TOKEN_PROGRAM_ID,
            );

            expect(result.mint.address.toBase58()).toBe(mintAddress.toBase58());
            expect(result.mint.mintAuthority?.toBase58()).toBe(
                mintAuthority.toBase58(),
            );
            expect(result.mint.freezeAuthority?.toBase58()).toBe(
                freezeAuthority.toBase58(),
            );
            expect(result.mint.supply).toBe(1_000_000n);
            expect(result.mint.decimals).toBe(9);
            expect(result.mint.isInitialized).toBe(true);
            expect(result.programId.toBase58()).toBe(
                TOKEN_PROGRAM_ID.toBase58(),
            );
            expect(result.mintContext).toBeUndefined();
            expect(result.tokenMetadata).toBeUndefined();
        });

        it('should unpack SPL mint with null authorities', () => {
            const mintAddress = Keypair.generate().publicKey;

            const buffer = Buffer.alloc(MINT_SIZE);
            MintLayout.encode(
                {
                    mintAuthorityOption: 0,
                    mintAuthority: PublicKey.default,
                    supply: BigInt(0),
                    decimals: 6,
                    isInitialized: true,
                    freezeAuthorityOption: 0,
                    freezeAuthority: PublicKey.default,
                },
                buffer,
            );

            const accountInfo: AccountInfo<Buffer> = {
                data: buffer,
                executable: false,
                lamports: 1_000_000,
                owner: TOKEN_PROGRAM_ID,
                rentEpoch: 0,
            };

            const result = unpackMintInterface(
                mintAddress,
                accountInfo,
                TOKEN_PROGRAM_ID,
            );

            expect(result.mint.mintAuthority).toBeNull();
            expect(result.mint.freezeAuthority).toBeNull();
        });
    });

    describe('Token-2022 mint', () => {
        it('should unpack Token-2022 mint data', () => {
            const mintAddress = Keypair.generate().publicKey;
            const mintAuthority = Keypair.generate().publicKey;

            const buffer = Buffer.alloc(MINT_SIZE);
            MintLayout.encode(
                {
                    mintAuthorityOption: 1,
                    mintAuthority,
                    supply: BigInt(500_000),
                    decimals: 6,
                    isInitialized: true,
                    freezeAuthorityOption: 0,
                    freezeAuthority: PublicKey.default,
                },
                buffer,
            );

            const accountInfo: AccountInfo<Buffer> = {
                data: buffer,
                executable: false,
                lamports: 1_000_000,
                owner: TOKEN_2022_PROGRAM_ID,
                rentEpoch: 0,
            };

            const result = unpackMintInterface(
                mintAddress,
                accountInfo,
                TOKEN_2022_PROGRAM_ID,
            );

            expect(result.mint.supply).toBe(500_000n);
            expect(result.mint.decimals).toBe(6);
            expect(result.programId.toBase58()).toBe(
                TOKEN_2022_PROGRAM_ID.toBase58(),
            );
        });
    });

    describe('CToken mint', () => {
        it('should unpack compressed mint data without extensions', () => {
            const mintAddress = Keypair.generate().publicKey;
            const mintAuthority = Keypair.generate().publicKey;
            const splMint = Keypair.generate().publicKey;

            const compressedMint: CompressedMint = {
                base: {
                    mintAuthority,
                    supply: BigInt(2_000_000),
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

            const buffer = serializeMint(compressedMint);

            const result = unpackMintInterface(
                mintAddress,
                buffer,
                LIGHT_TOKEN_PROGRAM_ID,
            );

            expect(result.mint.address.toBase58()).toBe(mintAddress.toBase58());
            expect(result.mint.mintAuthority?.toBase58()).toBe(
                mintAuthority.toBase58(),
            );
            expect(result.mint.supply).toBe(2_000_000n);
            expect(result.mint.decimals).toBe(9);
            expect(result.mint.isInitialized).toBe(true);
            expect(result.programId.toBase58()).toBe(
                LIGHT_TOKEN_PROGRAM_ID.toBase58(),
            );
            expect(result.mintContext).toBeDefined();
            expect(result.mintContext!.version).toBe(1);
            expect(result.mintContext!.cmintDecompressed).toBe(true);
            expect(result.mintContext!.splMint.toBase58()).toBe(
                splMint.toBase58(),
            );
            expect(result.tokenMetadata).toBeUndefined();
        });

        it('should unpack compressed mint data with TokenMetadata extension', () => {
            const mintAddress = Keypair.generate().publicKey;
            const mintAuthority = Keypair.generate().publicKey;
            const updateAuthority = Keypair.generate().publicKey;
            const splMint = Keypair.generate().publicKey;

            const metadata: TokenMetadata = {
                updateAuthority,
                mint: mintAddress,
                name: 'Test Token',
                symbol: 'TEST',
                uri: 'https://example.com/metadata.json',
            };

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
                    cmintDecompressed: false,
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

            const buffer = serializeMint(compressedMint);

            const result = unpackMintInterface(
                mintAddress,
                buffer,
                LIGHT_TOKEN_PROGRAM_ID,
            );

            expect(result.tokenMetadata).toBeDefined();
            expect(result.tokenMetadata!.name).toBe('Test Token');
            expect(result.tokenMetadata!.symbol).toBe('TEST');
            expect(result.tokenMetadata!.uri).toBe(
                'https://example.com/metadata.json',
            );
            expect(result.tokenMetadata!.updateAuthority?.toBase58()).toBe(
                updateAuthority.toBase58(),
            );
            expect(result.extensions).toBeDefined();
            expect(result.extensions!.length).toBe(1);
        });

        it('should handle Buffer input', () => {
            const mintAddress = Keypair.generate().publicKey;

            const compressedMint: CompressedMint = {
                base: {
                    mintAuthority: null,
                    supply: BigInt(100),
                    decimals: 6,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    cmintDecompressed: false,
                    splMint: PublicKey.default,
                    mintSigner: new Uint8Array(32),
                    bump: 254,
                },
                extensions: null,
            };

            const buffer = serializeMint(compressedMint);

            const result = unpackMintInterface(
                mintAddress,
                buffer,
                LIGHT_TOKEN_PROGRAM_ID,
            );

            expect(result.mint.supply).toBe(100n);
        });

        it('should handle Uint8Array input', () => {
            const mintAddress = Keypair.generate().publicKey;

            const compressedMint: CompressedMint = {
                base: {
                    mintAuthority: null,
                    supply: BigInt(200),
                    decimals: 9,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version: 1,
                    cmintDecompressed: false,
                    splMint: PublicKey.default,
                    mintSigner: new Uint8Array(32),
                    bump: 254,
                },
                extensions: null,
            };

            const buffer = serializeMint(compressedMint);
            const uint8Array = new Uint8Array(buffer);

            const result = unpackMintInterface(
                mintAddress,
                uint8Array,
                LIGHT_TOKEN_PROGRAM_ID,
            );

            expect(result.mint.supply).toBe(200n);
        });

        it('should default to TOKEN_PROGRAM_ID when no programId specified', () => {
            const mintAddress = Keypair.generate().publicKey;
            const mintAuthority = Keypair.generate().publicKey;

            const buffer = Buffer.alloc(MINT_SIZE);
            MintLayout.encode(
                {
                    mintAuthorityOption: 1,
                    mintAuthority,
                    supply: BigInt(1000),
                    decimals: 9,
                    isInitialized: true,
                    freezeAuthorityOption: 0,
                    freezeAuthority: PublicKey.default,
                },
                buffer,
            );

            const accountInfo: AccountInfo<Buffer> = {
                data: buffer,
                executable: false,
                lamports: 1_000_000,
                owner: TOKEN_PROGRAM_ID,
                rentEpoch: 0,
            };

            const result = unpackMintInterface(mintAddress, accountInfo);

            expect(result.programId.toBase58()).toBe(
                TOKEN_PROGRAM_ID.toBase58(),
            );
        });
    });
});

describe('unpackMintData', () => {
    it('should unpack compressed mint data and return mintContext', () => {
        const splMint = Keypair.generate().publicKey;

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
                cmintDecompressed: true,
                splMint,
                mintSigner: new Uint8Array(32),
                bump: 254,
            },
            extensions: null,
        };

        const buffer = serializeMint(compressedMint);
        const result = unpackMintData(buffer);

        expect(result.mintContext).toBeDefined();
        expect(result.mintContext.version).toBe(1);
        expect(result.mintContext.cmintDecompressed).toBe(true);
        expect(result.mintContext.splMint.toBase58()).toBe(splMint.toBase58());
        expect(result.tokenMetadata).toBeUndefined();
        expect(result.extensions).toBeUndefined();
    });

    it('should unpack compressed mint data with tokenMetadata', () => {
        const mintAddress = Keypair.generate().publicKey;
        const updateAuthority = Keypair.generate().publicKey;

        const metadata: TokenMetadata = {
            updateAuthority,
            mint: mintAddress,
            name: 'Unpack Test',
            symbol: 'UPK',
            uri: 'https://unpack.test/metadata.json',
            additionalMetadata: [{ key: 'version', value: '1.0' }],
        };

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
                cmintDecompressed: false,
                splMint: PublicKey.default,
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

        const buffer = serializeMint(compressedMint);
        const result = unpackMintData(buffer);

        expect(result.tokenMetadata).toBeDefined();
        expect(result.tokenMetadata!.name).toBe('Unpack Test');
        expect(result.tokenMetadata!.symbol).toBe('UPK');
        expect(result.tokenMetadata!.uri).toBe(
            'https://unpack.test/metadata.json',
        );
        expect(result.tokenMetadata!.updateAuthority?.toBase58()).toBe(
            updateAuthority.toBase58(),
        );
        expect(result.tokenMetadata!.additionalMetadata).toBeDefined();
        expect(result.tokenMetadata!.additionalMetadata!.length).toBe(1);
        expect(result.tokenMetadata!.additionalMetadata![0].key).toBe(
            'version',
        );
        expect(result.tokenMetadata!.additionalMetadata![0].value).toBe('1.0');
    });

    it('should return extensions array when present', () => {
        const mintAddress = Keypair.generate().publicKey;

        const metadata: TokenMetadata = {
            mint: mintAddress,
            name: 'Extensions Test',
            symbol: 'EXT',
            uri: 'https://ext.test',
        };

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
                cmintDecompressed: false,
                splMint: PublicKey.default,
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

        const buffer = serializeMint(compressedMint);
        const result = unpackMintData(buffer);

        expect(result.extensions).toBeDefined();
        expect(result.extensions!.length).toBe(1);
        expect(result.extensions![0].extensionType).toBe(
            ExtensionType.TokenMetadata,
        );
    });

    it('should handle Uint8Array input', () => {
        const compressedMint: CompressedMint = {
            base: {
                mintAuthority: null,
                supply: BigInt(0),
                decimals: 9,
                isInitialized: true,
                freezeAuthority: null,
            },
            mintContext: {
                version: 2,
                cmintDecompressed: false,
                splMint: PublicKey.default,
                mintSigner: new Uint8Array(32),
                bump: 254,
            },
            extensions: null,
        };

        const buffer = serializeMint(compressedMint);
        const uint8Array = new Uint8Array(buffer);
        const result = unpackMintData(uint8Array);

        expect(result.mintContext.version).toBe(2);
    });

    it('should handle different version values', () => {
        const versions = [0, 1, 127, 255];

        versions.forEach(version => {
            const compressedMint: CompressedMint = {
                base: {
                    mintAuthority: null,
                    supply: BigInt(0),
                    decimals: 9,
                    isInitialized: true,
                    freezeAuthority: null,
                },
                mintContext: {
                    version,
                    cmintDecompressed: false,
                    splMint: PublicKey.default,
                    mintSigner: new Uint8Array(32),
                    bump: 254,
                },
                extensions: null,
            };

            const buffer = serializeMint(compressedMint);
            const result = unpackMintData(buffer);

            expect(result.mintContext.version).toBe(version);
        });
    });

    it('should handle cmintDecompressed boolean correctly', () => {
        [true, false].forEach(initialized => {
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
                    cmintDecompressed: initialized,
                    splMint: PublicKey.default,
                    mintSigner: new Uint8Array(32),
                    bump: 254,
                },
                extensions: null,
            };

            const buffer = serializeMint(compressedMint);
            const result = unpackMintData(buffer);

            expect(result.mintContext.cmintDecompressed).toBe(initialized);
        });
    });

    it('should handle metadata without updateAuthority', () => {
        const mintAddress = Keypair.generate().publicKey;

        const metadata: TokenMetadata = {
            mint: mintAddress,
            name: 'No Authority',
            symbol: 'NA',
            uri: 'https://na.test',
        };

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
                cmintDecompressed: false,
                splMint: PublicKey.default,
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

        const buffer = serializeMint(compressedMint);
        const result = unpackMintData(buffer);

        expect(result.tokenMetadata).toBeDefined();
        expect(result.tokenMetadata!.updateAuthority).toBeUndefined();
    });
});
