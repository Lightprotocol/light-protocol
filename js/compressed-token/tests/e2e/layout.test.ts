import { describe, it, expect } from 'vitest';
import { PublicKey } from '@solana/web3.js';
import {
    Program,
    AnchorProvider,
    setProvider,
    Wallet,
} from '@coral-xyz/anchor';
import BN from 'bn.js';
import {
    encodeMintToInstructionData,
    decodeMintToInstructionData,
    encodeCompressSplTokenAccountInstructionData,
    decodeCompressSplTokenAccountInstructionData,
    encodeTransferInstructionData,
    decodeTransferInstructionData,
    IDL,
    LightCompressedToken,
    mintToAccountsLayout,
    createTokenPoolAccountsLayout,
    transferAccountsLayout,
    CompressedTokenProgram,
} from '../../src/';
import { Keypair } from '@solana/web3.js';
import { Connection } from '@solana/web3.js';
import { TOKEN_2022_PROGRAM_ID } from '@solana/spl-token';
import { SystemProgram } from '@solana/web3.js';
import {
    defaultStaticAccountsStruct,
    LightSystemProgram,
} from '@lightprotocol/stateless.js';

const getTestProgram = (): Program<LightCompressedToken> => {
    const mockKeypair = Keypair.generate();
    const mockConnection = new Connection('http://127.0.0.1:8899', 'confirmed');
    const mockProvider = new AnchorProvider(
        mockConnection,
        new Wallet(mockKeypair),
        {
            commitment: 'confirmed',
        },
    );
    setProvider(mockProvider);
    return new Program(
        IDL,
        new PublicKey('cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m'),
        mockProvider,
    );
};

function deepEqual(ref: any, val: any) {
    if (typeof ref !== typeof val) {
        console.log(`Type mismatch: ${typeof ref} !== ${typeof val}`);
        return false;
    }

    if (ref instanceof BN && val instanceof BN) {
        return ref.eq(val);
    }

    if (typeof ref === 'object' && ref !== null && val !== null) {
        const refKeys = Object.keys(ref);
        const valKeys = Object.keys(val);

        if (refKeys.length !== valKeys.length) {
            console.log(
                `Key length mismatch: ${refKeys.length} !== ${valKeys.length}`,
            );
            return false;
        }

        for (const key of refKeys) {
            if (!valKeys.includes(key)) {
                console.log(`Key ${key} not found in value`);
                return false;
            }
            if (!deepEqual(ref[key], val[key])) {
                console.log(`Value mismatch at key ${key}`);
                return false;
            }
        }
        return true;
    }

    if (ref !== val) {
        console.log(`Value mismatch: ${ref} !== ${val}`);
    }

    return ref === val;
}

const IX_DISCRIMINATOR = 8;
const LENGTH_DISCRIMINATOR = 4;

describe('layout', () => {
    const mint = Keypair.generate().publicKey;
    const feePayer = Keypair.generate().publicKey;
    const authority = Keypair.generate().publicKey;
    const cpiAuthorityPda = CompressedTokenProgram.deriveCpiAuthorityPda;
    const tokenPoolPda = CompressedTokenProgram.deriveTokenPoolPda(mint);
    const tokenProgram = TOKEN_2022_PROGRAM_ID;
    const lightSystemProgram = LightSystemProgram.programId;
    const registeredProgramPda =
        defaultStaticAccountsStruct().registeredProgramPda;
    const noopProgram = defaultStaticAccountsStruct().noopProgram;
    const accountCompressionAuthority =
        defaultStaticAccountsStruct().accountCompressionAuthority;
    const accountCompressionProgram =
        defaultStaticAccountsStruct().accountCompressionProgram;
    const merkleTree = PublicKey.default;
    const selfProgram = CompressedTokenProgram.programId;
    const systemProgram = SystemProgram.programId;
    const solPoolPda = LightSystemProgram.deriveCompressedSolPda();
    describe('encode/decode transfer/compress/decompress', () => {
        const testCases = [
            {
                description: 'default case',
                data: {
                    proof: {
                        a: [
                            32, 3, 117, 58, 153, 131, 148, 196, 202, 221, 250,
                            146, 196, 209, 8, 192, 211, 235, 57, 47, 234, 98,
                            152, 195, 227, 9, 16, 156, 194, 41, 247, 89,
                        ],
                        b: [
                            22, 192, 18, 134, 24, 94, 169, 42, 151, 182, 237,
                            164, 250, 163, 253, 24, 51, 142, 37, 55, 141, 92,
                            198, 146, 177, 23, 113, 12, 122, 27, 143, 64, 26,
                            191, 99, 235, 113, 154, 23, 234, 173, 101, 16, 34,
                            192, 108, 61, 10, 206, 251, 84, 242, 238, 92, 131,
                            107, 252, 227, 70, 181, 35, 236, 195, 209,
                        ],
                        c: [
                            166, 160, 56, 185, 41, 239, 140, 4, 255, 144, 213,
                            185, 153, 246, 199, 206, 47, 210, 17, 10, 66, 68,
                            132, 229, 12, 67, 166, 168, 229, 156, 90, 30,
                        ],
                    },
                    mint: new PublicKey(
                        'Bwuvv7NXd59zXRvWRCXcPLvwZ2dfedyQ9XZyqDghRFxv',
                    ),
                    delegatedTransfer: null,
                    inputTokenDataWithContext: [
                        {
                            amount: new BN('03e8', 16),
                            delegateIndex: null,
                            merkleContext: {
                                merkleTreePubkeyIndex: 0,
                                nullifierQueuePubkeyIndex: 1,
                                leafIndex: 10,
                                queueIndex: null,
                            },
                            rootIndex: 11,
                            lamports: null,
                            tlv: null,
                        },
                    ],
                    outputCompressedAccounts: [
                        {
                            owner: new PublicKey(
                                'ARaDUvjovQDvFTMqaNAu9f2j1MpqJ5rhDAnDFrnyKbwg',
                            ),
                            amount: new BN('012c', 16),
                            lamports: null,
                            merkleTreeIndex: 0,
                            tlv: null,
                        },
                        {
                            owner: new PublicKey(
                                'GWYLPLzCCAVxq12UvBSpU4F8pcsmmRYQobPxkGz67ZVx',
                            ),
                            amount: new BN('02bc', 16),
                            lamports: null,
                            merkleTreeIndex: 0,
                            tlv: null,
                        },
                    ],
                    compressOrDecompressAmount: null,
                    isCompress: false,
                    cpiContext: null,
                    lamportsChangeAccountMerkleTreeIndex: null,
                },
            },
            {
                description: 'with compressOrDecompressAmount',
                data: {
                    proof: null,
                    mint: new PublicKey(
                        'Bwuvv7NXd59zXRvWRCXcPLvwZ2dfedyQ9XZyqDghRFxv',
                    ),
                    delegatedTransfer: null,
                    inputTokenDataWithContext: [],
                    outputCompressedAccounts: [],
                    compressOrDecompressAmount: new BN(500),
                    isCompress: true,
                    cpiContext: null,
                    lamportsChangeAccountMerkleTreeIndex: null,
                },
            },
            {
                description: 'with delegatedTransfer',
                data: {
                    proof: null,
                    mint: new PublicKey(
                        'Bwuvv7NXd59zXRvWRCXcPLvwZ2dfedyQ9XZyqDghRFxv',
                    ),
                    delegatedTransfer: {
                        owner: new PublicKey(
                            'ARaDUvjovQDvFTMqaNAu9f2j1MpqJ5rhDAnDFrnyKbwg',
                        ),
                        delegateChangeAccountIndex: 1,
                    },
                    inputTokenDataWithContext: [],
                    outputCompressedAccounts: [],
                    compressOrDecompressAmount: null,
                    isCompress: false,
                    cpiContext: null,
                    lamportsChangeAccountMerkleTreeIndex: null,
                },
            },
            {
                description: 'with proof none',
                data: {
                    proof: null,
                    mint: new PublicKey(
                        'Bwuvv7NXd59zXRvWRCXcPLvwZ2dfedyQ9XZyqDghRFxv',
                    ),
                    delegatedTransfer: null,
                    inputTokenDataWithContext: [],
                    outputCompressedAccounts: [],
                    compressOrDecompressAmount: null,
                    isCompress: false,
                    cpiContext: null,
                    lamportsChangeAccountMerkleTreeIndex: null,
                },
            },
            {
                description: 'with various inputTokenDataWithContext',
                data: {
                    proof: null,
                    mint: new PublicKey(
                        'Bwuvv7NXd59zXRvWRCXcPLvwZ2dfedyQ9XZyqDghRFxv',
                    ),
                    delegatedTransfer: null,
                    inputTokenDataWithContext: [
                        {
                            amount: new BN(1000),
                            delegateIndex: 2,
                            merkleContext: {
                                merkleTreePubkeyIndex: 1,
                                nullifierQueuePubkeyIndex: 2,
                                leafIndex: 3,
                                queueIndex: { queueId: 0, index: 4 },
                            },
                            rootIndex: 5,
                            lamports: new BN(2000),
                            tlv: Buffer.from([1, 2, 3]),
                        },
                    ],
                    outputCompressedAccounts: [],
                    compressOrDecompressAmount: null,
                    isCompress: false,
                    cpiContext: null,
                    lamportsChangeAccountMerkleTreeIndex: null,
                },
            },
            {
                description: 'with various outputCompressedAccounts',
                data: {
                    proof: null,
                    mint: new PublicKey(
                        'Bwuvv7NXd59zXRvWRCXcPLvwZ2dfedyQ9XZyqDghRFxv',
                    ),
                    delegatedTransfer: null,
                    inputTokenDataWithContext: [],
                    outputCompressedAccounts: [
                        {
                            owner: new PublicKey(
                                'ARaDUvjovQDvFTMqaNAu9f2j1MpqJ5rhDAnDFrnyKbwg',
                            ),
                            amount: new BN(3000),
                            lamports: new BN(4000),
                            merkleTreeIndex: 1,
                            tlv: Buffer.from([4, 5, 6]),
                        },
                    ],
                    compressOrDecompressAmount: null,
                    isCompress: false,
                    cpiContext: null,
                    lamportsChangeAccountMerkleTreeIndex: null,
                },
            },
            {
                description: 'with isCompress true',
                data: {
                    proof: null,
                    mint: new PublicKey(
                        'Bwuvv7NXd59zXRvWRCXcPLvwZ2dfedyQ9XZyqDghRFxv',
                    ),
                    delegatedTransfer: null,
                    inputTokenDataWithContext: [],
                    outputCompressedAccounts: [],
                    compressOrDecompressAmount: null,
                    isCompress: true,
                    cpiContext: null,
                    lamportsChangeAccountMerkleTreeIndex: null,
                },
            },
            {
                description: 'with lamportsChangeAccountMerkleTreeIndex',
                data: {
                    proof: null,
                    mint: new PublicKey(
                        'Bwuvv7NXd59zXRvWRCXcPLvwZ2dfedyQ9XZyqDghRFxv',
                    ),
                    delegatedTransfer: null,
                    inputTokenDataWithContext: [],
                    outputCompressedAccounts: [],
                    compressOrDecompressAmount: null,
                    isCompress: false,
                    cpiContext: null,
                    lamportsChangeAccountMerkleTreeIndex: 5,
                },
            },
            {
                description: 'with cpiContext',
                data: {
                    proof: null,
                    mint: new PublicKey(
                        'Bwuvv7NXd59zXRvWRCXcPLvwZ2dfedyQ9XZyqDghRFxv',
                    ),
                    delegatedTransfer: null,
                    inputTokenDataWithContext: [],
                    outputCompressedAccounts: [],
                    compressOrDecompressAmount: null,
                    isCompress: false,
                    cpiContext: {
                        setContext: true,
                        firstSetContext: false,
                        cpiContextAccountIndex: 2,
                    },
                    lamportsChangeAccountMerkleTreeIndex: null,
                },
            },
        ];

        testCases.forEach(({ description, data }) => {
            it(`should encode/decode transfer: ${description}`, () => {
                const anchorEncodedData = getTestProgram().coder.types.encode(
                    'CompressedTokenInstructionDataTransfer',
                    data,
                );
                const encoded = encodeTransferInstructionData(data);
                const decoded = decodeTransferInstructionData(encoded);
                expect(deepEqual(decoded, data)).toBe(true);
                expect(anchorEncodedData).toEqual(
                    encoded.slice(IX_DISCRIMINATOR + LENGTH_DISCRIMINATOR),
                );
            });
        });
    });

    describe('encode/decode MintToInstructionData', () => {
        const testCases = [
            {
                description: 'default case',
                data: {
                    recipients: [
                        new PublicKey(
                            '6ASf5EcmmEHTgDJ4X4ZT5vT6iHVJBXPg5AN5YoTCpGWt',
                        ),
                    ],
                    amounts: [new BN(1000)],
                    lamports: null,
                },
            },
            {
                description: 'with multiple recipients',
                data: {
                    recipients: [
                        new PublicKey(
                            '6ASf5EcmmEHTgDJ4X4ZT5vT6iHVJBXPg5AN5YoTCpGWt',
                        ),
                        new PublicKey(
                            '8ASf5EcmmEHTgDJ4X4ZT5vT6iHVJBXPg5AN5YoTCpGWs',
                        ),
                    ],
                    amounts: [new BN(1000), new BN(2000)],
                    lamports: null,
                },
            },
            {
                description: 'with lamports',
                data: {
                    recipients: [
                        new PublicKey(
                            '6ASf5EcmmEHTgDJ4X4ZT5vT6iHVJBXPg5AN5YoTCpGWt',
                        ),
                    ],
                    amounts: [new BN(1000)],
                    lamports: new BN(500),
                },
            },
        ];

        testCases.forEach(async ({ description, data }) => {
            it(description, async () => {
                const encoded = encodeMintToInstructionData(data);
                const decoded = decodeMintToInstructionData(encoded);
                expect(deepEqual(decoded, data)).toBe(true);

                const instruction = await getTestProgram()
                    .methods.mintTo(
                        data.recipients,
                        data.amounts,
                        data.lamports,
                    )
                    .accounts({
                        feePayer: PublicKey.default,
                        authority: PublicKey.default,
                        cpiAuthorityPda: PublicKey.default,
                        mint: PublicKey.default,
                        tokenPoolPda: PublicKey.default,
                        tokenProgram: PublicKey.default,
                        lightSystemProgram: PublicKey.default,
                        registeredProgramPda: PublicKey.default,
                        noopProgram: PublicKey.default,
                        accountCompressionAuthority: PublicKey.default,
                        accountCompressionProgram: PublicKey.default,
                        merkleTree: PublicKey.default,
                        selfProgram: PublicKey.default,
                        solPoolPda: null,
                    })
                    .instruction();
                expect(instruction.data).toEqual(encoded);
            });
        });
    });

    describe('encode/decode CompressSplTokenAccountInstructionData', () => {
        const testCases = [
            {
                description: 'default case',
                data: {
                    owner: new PublicKey(
                        'CPMzHV9PsUeb5pFmyrj9nEoDwtL8CcyUKQzJXJxYRnT7',
                    ),
                    remainingAmount: new BN(110),
                    cpiContext: null,
                },
            },
            {
                description: 'with cpiContext',
                data: {
                    owner: new PublicKey(
                        'CPMzHV9PsUeb5pFmyrj9nEoDwtL8CcyUKQzJXJxYRnT7',
                    ),
                    remainingAmount: new BN(110),
                    cpiContext: {
                        setContext: true,
                        firstSetContext: true,
                        cpiContextAccountIndex: 0,
                    },
                },
            },
            {
                description: 'without remainingAmount',
                data: {
                    owner: new PublicKey(
                        'CPMzHV9PsUeb5pFmyrj9nEoDwtL8CcyUKQzJXJxYRnT7',
                    ),
                    remainingAmount: null,
                    cpiContext: null,
                },
            },
        ];

        testCases.forEach(async ({ description, data }) => {
            it(description, async () => {
                const encoded =
                    encodeCompressSplTokenAccountInstructionData(data);
                const decoded =
                    decodeCompressSplTokenAccountInstructionData(encoded);
                expect(deepEqual(decoded, data)).toBe(true);

                const instruction = await getTestProgram()
                    .methods.compressSplTokenAccount(
                        data.owner,
                        data.remainingAmount,
                        data.cpiContext,
                    )
                    .accounts({
                        feePayer: PublicKey.default,
                        authority: PublicKey.default,
                        cpiAuthorityPda: PublicKey.default,
                        lightSystemProgram: PublicKey.default,
                        registeredProgramPda: PublicKey.default,
                        noopProgram: PublicKey.default,
                        accountCompressionAuthority: PublicKey.default,
                        accountCompressionProgram: PublicKey.default,
                        selfProgram: PublicKey.default,
                        tokenPoolPda: PublicKey.default,
                        compressOrDecompressTokenAccount: PublicKey.default,
                        tokenProgram: PublicKey.default,
                        systemProgram: PublicKey.default,
                    })
                    .instruction();

                expect(instruction.data).toEqual(encoded);
            });
        });
    });
    describe('Accounts Layout Helper Functions', () => {
        it('createTokenPoolAccountsLayout should return correct AccountMeta array', () => {
            const accounts = {
                feePayer,
                tokenPoolPda,
                systemProgram,
                mint,
                tokenProgram,
                cpiAuthorityPda,
            };

            const expected = [
                { pubkey: feePayer, isSigner: true, isWritable: true },
                {
                    pubkey: tokenPoolPda,
                    isSigner: false,
                    isWritable: true,
                },
                {
                    pubkey: systemProgram,
                    isSigner: false,
                    isWritable: false,
                },
                {
                    pubkey: mint,
                    isSigner: false,
                    isWritable: true,
                },
                {
                    pubkey: tokenProgram,
                    isSigner: false,
                    isWritable: false,
                },
                {
                    pubkey: cpiAuthorityPda,
                    isSigner: false,
                    isWritable: false,
                },
            ];

            const result = createTokenPoolAccountsLayout(accounts);
            expect(result).toEqual(expected);
        });

        it('mintToAccountsLayout should return correct AccountMeta array', () => {
            const accounts = {
                feePayer,
                authority,
                cpiAuthorityPda,
                mint,
                tokenPoolPda,
                tokenProgram,
                lightSystemProgram,
                registeredProgramPda,
                noopProgram,
                accountCompressionAuthority,
                accountCompressionProgram,
                merkleTree,
                selfProgram,
                systemProgram,
                solPoolPda,
            };

            const expected = [
                { pubkey: feePayer, isSigner: true, isWritable: true },
                {
                    pubkey: authority,
                    isSigner: true,
                    isWritable: false,
                },
                {
                    pubkey: cpiAuthorityPda,
                    isSigner: false,
                    isWritable: false,
                },
                {
                    pubkey: mint,
                    isSigner: false,
                    isWritable: true,
                },
                {
                    pubkey: tokenPoolPda,
                    isSigner: false,
                    isWritable: true,
                },
                {
                    pubkey: tokenProgram,
                    isSigner: false,
                    isWritable: false,
                },
                {
                    pubkey: lightSystemProgram,
                    isSigner: false,
                    isWritable: false,
                },
                {
                    pubkey: registeredProgramPda,
                    isSigner: false,
                    isWritable: false,
                },
                {
                    pubkey: noopProgram,
                    isSigner: false,
                    isWritable: false,
                },
                {
                    pubkey: accountCompressionAuthority,
                    isSigner: false,
                    isWritable: false,
                },
                {
                    pubkey: accountCompressionProgram,
                    isSigner: false,
                    isWritable: false,
                },
                {
                    pubkey: merkleTree,
                    isSigner: false,
                    isWritable: true,
                },
                {
                    pubkey: selfProgram,
                    isSigner: false,
                    isWritable: false,
                },
                {
                    pubkey: systemProgram,
                    isSigner: false,
                    isWritable: false,
                },
                {
                    pubkey: solPoolPda,
                    isSigner: false,
                    isWritable: true,
                },
            ];

            const result = mintToAccountsLayout(accounts);
            expect(result).toEqual(expected);
        });

        it('transferAccountsLayout should return correct AccountMeta array', () => {
            const compressOrDecompressTokenAccount =
                Keypair.generate().publicKey;
            const accounts = {
                feePayer,
                authority,
                cpiAuthorityPda,
                lightSystemProgram,
                registeredProgramPda,
                noopProgram,
                accountCompressionAuthority,
                accountCompressionProgram,
                selfProgram,
                tokenPoolPda,
                compressOrDecompressTokenAccount,
                tokenProgram,
                systemProgram,
            };

            const expected = [
                { pubkey: feePayer, isSigner: true, isWritable: true },
                {
                    pubkey: authority,
                    isSigner: true,
                    isWritable: false,
                },
                {
                    pubkey: cpiAuthorityPda,
                    isSigner: false,
                    isWritable: false,
                },
                {
                    pubkey: lightSystemProgram,
                    isSigner: false,
                    isWritable: false,
                },
                {
                    pubkey: registeredProgramPda,
                    isSigner: false,
                    isWritable: false,
                },
                {
                    pubkey: noopProgram,
                    isSigner: false,
                    isWritable: false,
                },
                {
                    pubkey: accountCompressionAuthority,
                    isSigner: false,
                    isWritable: false,
                },
                {
                    pubkey: accountCompressionProgram,
                    isSigner: false,
                    isWritable: false,
                },
                {
                    pubkey: selfProgram,
                    isSigner: false,
                    isWritable: false,
                },
                {
                    pubkey: tokenPoolPda,
                    isSigner: false,
                    isWritable: true,
                },
                {
                    pubkey: compressOrDecompressTokenAccount,
                    isSigner: false,
                    isWritable: true,
                },
                {
                    pubkey: tokenProgram,
                    isSigner: false,
                    isWritable: false,
                },
                {
                    pubkey: systemProgram,
                    isSigner: false,
                    isWritable: false,
                },
            ];

            const result = transferAccountsLayout(accounts);
            expect(result).toEqual(expected);
        });
    });
});
