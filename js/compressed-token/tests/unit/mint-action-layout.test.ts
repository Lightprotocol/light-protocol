import { describe, it, expect } from 'vitest';
import { PublicKey, Keypair } from '@solana/web3.js';
import { Buffer } from 'buffer';
import {
    encodeMintActionInstructionData,
    decodeMintActionInstructionData,
    MintActionCompressedInstructionData,
    MINT_ACTION_DISCRIMINATOR,
} from '../../src/v3/layout/layout-mint-action';
import { encodeCreateMintInstructionData } from '../../src/v3/instructions/create-mint';
import { TokenDataVersion } from '../../src/constants';
import {
    deriveAddressV2,
    CTOKEN_PROGRAM_ID,
    getBatchAddressTreeInfo,
} from '@lightprotocol/stateless.js';
import { findMintAddress } from '../../src/v3/derivation';

describe('MintActionCompressedInstructionData Layout', () => {
    describe('encode/decode round-trip', () => {
        it('should encode and decode createMint instruction data correctly', () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();

            // Create data matching what encodeCreateMintInstructionData produces
            const instructionData: MintActionCompressedInstructionData = {
                leafIndex: 0,
                proveByIndex: false,
                rootIndex: 42,
                compressedAddress: Array.from(new Uint8Array(32).fill(1)),
                tokenPoolBump: 0,
                tokenPoolIndex: 0,
                maxTopUp: 0,
                createMint: {
                    readOnlyAddressTrees: [0, 0, 0, 0],
                    readOnlyAddressTreeRootIndices: [0, 0, 0, 0],
                },
                actions: [],
                proof: {
                    a: Array.from(new Uint8Array(32).fill(2)),
                    b: Array.from(new Uint8Array(64).fill(3)),
                    c: Array.from(new Uint8Array(32).fill(4)),
                },
                cpiContext: null,
                mint: {
                    supply: BigInt(0),
                    decimals: 9,
                    metadata: {
                        version: TokenDataVersion.ShaFlat,
                        cmintDecompressed: false,
                        mint: mintSigner.publicKey,
                    },
                    mintAuthority: mintAuthority.publicKey,
                    freezeAuthority: null,
                    extensions: null,
                },
            };

            const encoded = encodeMintActionInstructionData(instructionData);
            expect(encoded[0]).toBe(103); // MINT_ACTION_DISCRIMINATOR

            const decoded = decodeMintActionInstructionData(encoded);

            // Verify all fields match
            expect(decoded.leafIndex).toBe(instructionData.leafIndex);
            expect(decoded.proveByIndex).toBe(instructionData.proveByIndex);
            expect(decoded.rootIndex).toBe(instructionData.rootIndex);
            expect(decoded.compressedAddress).toEqual(
                instructionData.compressedAddress,
            );
            expect(decoded.tokenPoolBump).toBe(instructionData.tokenPoolBump);
            expect(decoded.tokenPoolIndex).toBe(instructionData.tokenPoolIndex);
            expect(decoded.maxTopUp).toBe(instructionData.maxTopUp);
            expect(decoded.createMint).toEqual(instructionData.createMint);
            expect(decoded.actions).toEqual([]);
            expect(decoded.proof).toEqual(instructionData.proof);
            expect(decoded.cpiContext).toBeNull();
            expect(decoded.mint).toBeDefined();
            expect(decoded.mint!.decimals).toBe(9);
        });

        it('should encode createMint without proof (null proof)', () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();

            const instructionData: MintActionCompressedInstructionData = {
                leafIndex: 0,
                proveByIndex: false,
                rootIndex: 0,
                compressedAddress: Array.from(new Uint8Array(32).fill(0)),
                tokenPoolBump: 0,
                tokenPoolIndex: 0,
                maxTopUp: 0,
                createMint: {
                    readOnlyAddressTrees: [0, 0, 0, 0],
                    readOnlyAddressTreeRootIndices: [0, 0, 0, 0],
                },
                actions: [],
                proof: null,
                cpiContext: null,
                mint: {
                    supply: BigInt(0),
                    decimals: 6,
                    metadata: {
                        version: TokenDataVersion.ShaFlat,
                        cmintDecompressed: false,
                        mint: mintSigner.publicKey,
                    },
                    mintAuthority: mintAuthority.publicKey,
                    freezeAuthority: null,
                    extensions: null,
                },
            };

            const encoded = encodeMintActionInstructionData(instructionData);
            const decoded = decodeMintActionInstructionData(encoded);

            expect(decoded.proof).toBeNull();
            expect(decoded.mint!.decimals).toBe(6);
        });

        it('should encode createMint with freeze authority', () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();
            const freezeAuthority = Keypair.generate();

            const instructionData: MintActionCompressedInstructionData = {
                leafIndex: 0,
                proveByIndex: false,
                rootIndex: 100,
                compressedAddress: Array.from(new Uint8Array(32).fill(5)),
                tokenPoolBump: 0,
                tokenPoolIndex: 0,
                maxTopUp: 0,
                createMint: {
                    readOnlyAddressTrees: [0, 0, 0, 0],
                    readOnlyAddressTreeRootIndices: [0, 0, 0, 0],
                },
                actions: [],
                proof: {
                    a: Array.from(new Uint8Array(32).fill(10)),
                    b: Array.from(new Uint8Array(64).fill(11)),
                    c: Array.from(new Uint8Array(32).fill(12)),
                },
                cpiContext: null,
                mint: {
                    supply: BigInt(0),
                    decimals: 9,
                    metadata: {
                        version: TokenDataVersion.ShaFlat,
                        cmintDecompressed: false,
                        mint: mintSigner.publicKey,
                    },
                    mintAuthority: mintAuthority.publicKey,
                    freezeAuthority: freezeAuthority.publicKey,
                    extensions: null,
                },
            };

            const encoded = encodeMintActionInstructionData(instructionData);
            const decoded = decodeMintActionInstructionData(encoded);

            expect(decoded.mint!.freezeAuthority).toBeDefined();
            expect(decoded.mint!.freezeAuthority!.toBase58()).toBe(
                freezeAuthority.publicKey.toBase58(),
            );
        });

        it('should encode createMint with token metadata extension', () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();

            const instructionData: MintActionCompressedInstructionData = {
                leafIndex: 0,
                proveByIndex: false,
                rootIndex: 50,
                compressedAddress: Array.from(new Uint8Array(32).fill(6)),
                tokenPoolBump: 0,
                tokenPoolIndex: 0,
                maxTopUp: 0,
                createMint: {
                    readOnlyAddressTrees: [0, 0, 0, 0],
                    readOnlyAddressTreeRootIndices: [0, 0, 0, 0],
                },
                actions: [],
                proof: {
                    a: Array.from(new Uint8Array(32).fill(20)),
                    b: Array.from(new Uint8Array(64).fill(21)),
                    c: Array.from(new Uint8Array(32).fill(22)),
                },
                cpiContext: null,
                mint: {
                    supply: BigInt(0),
                    decimals: 9,
                    metadata: {
                        version: TokenDataVersion.ShaFlat,
                        cmintDecompressed: false,
                        mint: mintSigner.publicKey,
                    },
                    mintAuthority: mintAuthority.publicKey,
                    freezeAuthority: null,
                    extensions: [
                        {
                            tokenMetadata: {
                                updateAuthority: mintAuthority.publicKey,
                                name: Buffer.from('Test Token'),
                                symbol: Buffer.from('TEST'),
                                uri: Buffer.from(
                                    'https://test.com/metadata.json',
                                ),
                                additionalMetadata: null,
                            },
                        },
                    ],
                },
            };

            const encoded = encodeMintActionInstructionData(instructionData);
            const decoded = decodeMintActionInstructionData(encoded);

            expect(decoded.mint!.extensions).toBeDefined();
            expect(decoded.mint!.extensions!.length).toBe(1);
        });
    });

    describe('byte layout verification', () => {
        it('should have correct discriminator', () => {
            expect(MINT_ACTION_DISCRIMINATOR).toEqual(Buffer.from([103]));
        });

        it('should produce consistent byte output', () => {
            const mintSigner = PublicKey.default;
            const mintAuthority = Keypair.generate().publicKey;

            const instructionData: MintActionCompressedInstructionData = {
                leafIndex: 0,
                proveByIndex: false,
                rootIndex: 0,
                compressedAddress: Array(32).fill(0),
                tokenPoolBump: 0,
                tokenPoolIndex: 0,
                maxTopUp: 0,
                createMint: {
                    readOnlyAddressTrees: [0, 0, 0, 0],
                    readOnlyAddressTreeRootIndices: [0, 0, 0, 0],
                },
                actions: [],
                proof: null,
                cpiContext: null,
                mint: {
                    supply: BigInt(0),
                    decimals: 9,
                    metadata: {
                        version: 0,
                        cmintDecompressed: false,
                        mint: mintSigner,
                    },
                    mintAuthority: mintAuthority,
                    freezeAuthority: null,
                    extensions: null,
                },
            };

            const encoded1 = encodeMintActionInstructionData(instructionData);
            const encoded2 = encodeMintActionInstructionData(instructionData);

            // Should be deterministic
            expect(encoded1).toEqual(encoded2);

            // Log hex for debugging
            console.log(
                'Encoded bytes (hex):',
                encoded1.toString('hex').slice(0, 200) + '...',
            );
            console.log('Total encoded length:', encoded1.length);

            // First byte should be discriminator 103
            expect(encoded1[0]).toBe(103);

            // Next 4 bytes should be leafIndex (0 as u32 little-endian)
            expect(encoded1.slice(1, 5)).toEqual(Buffer.from([0, 0, 0, 0]));

            // Next byte should be proveByIndex (false = 0)
            expect(encoded1[5]).toBe(0);

            // Next 2 bytes should be rootIndex (0 as u16 little-endian)
            expect(encoded1.slice(6, 8)).toEqual(Buffer.from([0, 0]));

            // Next 32 bytes should be compressedAddress (all zeros)
            expect(encoded1.slice(8, 40)).toEqual(Buffer.alloc(32, 0));

            // tokenPoolBump at byte 40
            expect(encoded1[40]).toBe(0);

            // tokenPoolIndex at byte 41
            expect(encoded1[41]).toBe(0);

            // maxTopUp at bytes 42-43 (u16 little-endian)
            expect(encoded1.slice(42, 44)).toEqual(Buffer.from([0, 0]));

            // createMint Option: byte 44 should be 1 (Some)
            expect(encoded1[44]).toBe(1);

            // createMint.readOnlyAddressTrees: bytes 45-48
            expect(encoded1.slice(45, 49)).toEqual(Buffer.from([0, 0, 0, 0]));

            // createMint.readOnlyAddressTreeRootIndices: bytes 49-56 (4 x u16)
            expect(encoded1.slice(49, 57)).toEqual(
                Buffer.from([0, 0, 0, 0, 0, 0, 0, 0]),
            );
        });
    });

    describe('encodeCreateMintInstructionData (integration)', () => {
        it('should correctly encode create mint instruction data from params', () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();
            const addressTreeInfo = getBatchAddressTreeInfo();

            // Encode using the actual instruction builder
            const encoded = encodeCreateMintInstructionData({
                mintSigner: mintSigner.publicKey,
                mintAuthority: mintAuthority.publicKey,
                freezeAuthority: null,
                decimals: 9,
                addressTree: addressTreeInfo.tree,
                outputQueue: addressTreeInfo.queue,
                rootIndex: 42,
                proof: {
                    a: Array.from(new Uint8Array(32).fill(1)),
                    b: Array.from(new Uint8Array(64).fill(2)),
                    c: Array.from(new Uint8Array(32).fill(3)),
                },
            });

            // Discriminator check
            expect(encoded[0]).toBe(103);

            // Should be decodable
            const decoded = decodeMintActionInstructionData(encoded);
            expect(decoded.leafIndex).toBe(0);
            expect(decoded.proveByIndex).toBe(false);
            expect(decoded.rootIndex).toBe(42);
            expect(decoded.createMint).not.toBeNull();
            expect(decoded.mint).not.toBeNull();
            expect(decoded.mint!.decimals).toBe(9);
        });

        it('should correctly derive compressed mint address', () => {
            const mintSigner = Keypair.generate();
            const addressTreeInfo = getBatchAddressTreeInfo();

            // Get the mint PDA
            const [mintPda] = findMintAddress(mintSigner.publicKey);

            // Derive the compressed address the same way createMintInterface does
            const compressedAddress = deriveAddressV2(
                mintPda.toBytes(),
                addressTreeInfo.tree,
                CTOKEN_PROGRAM_ID,
            );

            // Verify it's a valid 32-byte address
            expect(compressedAddress.toBytes().length).toBe(32);
        });

        it('should encode create mint with null proof', () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();
            const addressTreeInfo = getBatchAddressTreeInfo();

            const encoded = encodeCreateMintInstructionData({
                mintSigner: mintSigner.publicKey,
                mintAuthority: mintAuthority.publicKey,
                freezeAuthority: null,
                decimals: 6,
                addressTree: addressTreeInfo.tree,
                outputQueue: addressTreeInfo.queue,
                rootIndex: 0,
                proof: null,
            });

            const decoded = decodeMintActionInstructionData(encoded);
            expect(decoded.proof).toBeNull();
        });

        it('should encode create mint with metadata', () => {
            const mintSigner = Keypair.generate();
            const mintAuthority = Keypair.generate();
            const addressTreeInfo = getBatchAddressTreeInfo();

            const encoded = encodeCreateMintInstructionData({
                mintSigner: mintSigner.publicKey,
                mintAuthority: mintAuthority.publicKey,
                freezeAuthority: null,
                decimals: 9,
                addressTree: addressTreeInfo.tree,
                outputQueue: addressTreeInfo.queue,
                rootIndex: 100,
                proof: {
                    a: Array.from(new Uint8Array(32).fill(10)),
                    b: Array.from(new Uint8Array(64).fill(20)),
                    c: Array.from(new Uint8Array(32).fill(30)),
                },
                metadata: {
                    name: 'Test Token',
                    symbol: 'TEST',
                    uri: 'https://test.com/metadata.json',
                    updateAuthority: mintAuthority.publicKey,
                    additionalMetadata: null,
                },
            });

            const decoded = decodeMintActionInstructionData(encoded);
            expect(decoded.mint!.extensions).not.toBeNull();
            expect(decoded.mint!.extensions!.length).toBe(1);
        });
    });
});
