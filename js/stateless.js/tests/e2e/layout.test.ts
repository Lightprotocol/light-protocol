import { describe, it, expect } from 'vitest';
import { Connection, Keypair, PublicKey, SystemProgram } from '@solana/web3.js';
import BN from 'bn.js';
import {
    Program,
    AnchorProvider,
    setProvider,
    Wallet,
} from '@coral-xyz/anchor';
import {
    encodeInstructionDataInvoke,
    decodeInstructionDataInvoke,
    encodePublicTransactionEvent,
    decodePublicTransactionEvent,
    invokeAccountsLayout,
} from '../../src/programs/system/layout';
import { PublicTransactionEvent } from '../../src/state';

import {
    COMPRESSED_TOKEN_PROGRAM_ID,
    defaultStaticAccountsStruct,
    IDL,
    LightSystemProgramIDL,
} from '../../src';
import { LightSystemProgram } from '../../src/programs/system';

const getTestProgram = (): Program<LightSystemProgramIDL> => {
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
    return new Program(IDL, COMPRESSED_TOKEN_PROGRAM_ID, mockProvider);
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

describe('layout', () => {
    describe('encode/decode InstructionDataInvoke', () => {
        const testCases = [
            {
                description: 'default case',
                data: {
                    proof: null,
                    inputCompressedAccountsWithMerkleContext: [],
                    outputCompressedAccounts: [],
                    relayFee: null,
                    newAddressParams: [],
                    compressOrDecompressLamports: null,
                    isCompress: true,
                },
            },
            {
                description: 'with proof',
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
                    inputCompressedAccountsWithMerkleContext: [],
                    outputCompressedAccounts: [],
                    relayFee: null,
                    newAddressParams: [],
                    compressOrDecompressLamports: null,
                    isCompress: true,
                },
            },
            {
                description: 'with inputCompressedAccountsWithMerkleContext',
                data: {
                    proof: null,
                    inputCompressedAccountsWithMerkleContext: [
                        {
                            compressedAccount: {
                                owner: new PublicKey(
                                    '6ASf5EcmmEHTgDJ4X4ZT5vT6iHVJBXPg5AN5YoTCpGWt',
                                ),
                                lamports: new BN(0),
                                address: null,
                                data: null,
                            },
                            merkleContext: {
                                merkleTreePubkeyIndex: 0,
                                queuePubkeyIndex: 1,
                                leafIndex: 10,
                                proveByIndex: false,
                            },
                            rootIndex: 0,
                            readOnly: false,
                        },
                    ],
                    outputCompressedAccounts: [],
                    relayFee: null,
                    newAddressParams: [],
                    compressOrDecompressLamports: null,
                    isCompress: true,
                },
            },
            {
                description: 'with outputCompressedAccounts',
                data: {
                    proof: null,
                    inputCompressedAccountsWithMerkleContext: [],
                    outputCompressedAccounts: [
                        {
                            compressedAccount: {
                                owner: new PublicKey(
                                    'ARaDUvjovQDvFTMqaNAu9f2j1MpqJ5rhDAnDFrnyKbwg',
                                ),
                                lamports: new BN(0),
                                address: null,
                                data: null,
                            },
                            merkleTreeIndex: 0,
                        },
                    ],
                    relayFee: null,
                    newAddressParams: [],
                    compressOrDecompressLamports: null,
                    isCompress: true,
                },
            },
            {
                description: 'with relayFee',
                data: {
                    proof: null,
                    inputCompressedAccountsWithMerkleContext: [],
                    outputCompressedAccounts: [],
                    relayFee: new BN(500),
                    newAddressParams: [],
                    compressOrDecompressLamports: null,
                    isCompress: true,
                },
            },
            {
                description: 'with newAddressParams',
                data: {
                    proof: null,
                    inputCompressedAccountsWithMerkleContext: [],
                    outputCompressedAccounts: [],
                    relayFee: null,
                    newAddressParams: [
                        {
                            seed: Array.from({ length: 32 }, () =>
                                Math.floor(Math.random() * 256),
                            ),
                            addressQueueAccountIndex: 0,
                            addressMerkleTreeAccountIndex: 0,
                            addressMerkleTreeRootIndex: 0,
                        },
                    ],
                    compressOrDecompressLamports: null,
                    isCompress: true,
                },
            },
        ];

        testCases.forEach(({ description, data }) => {
            it(`should encode/decode InstructionDataInvoke: ${description}`, () => {
                const encoded = encodeInstructionDataInvoke(data);
                const decoded = decodeInstructionDataInvoke(encoded);

                expect(deepEqual(decoded, data)).toBe(true);

                const anchordata = getTestProgram().coder.types.encode(
                    'InstructionDataInvoke',
                    data,
                );
                expect(anchordata).toEqual(encoded.slice(12));
            });
        });
    });

    describe('encode/decode PublicTransactionEvent', () => {
        it('should encode and decode PublicTransactionEvent correctly', () => {
            const data: PublicTransactionEvent = {
                inputCompressedAccountHashes: [],
                outputCompressedAccountHashes: [],
                outputCompressedAccounts: [],
                outputLeafIndices: [],
                sequenceNumbers: [],
                relayFee: null,
                isCompress: true,
                compressOrDecompressLamports: null,
                pubkeyArray: [],
                message: null,
            };

            const encoded = encodePublicTransactionEvent(data);

            const decoded = decodePublicTransactionEvent(encoded);

            const anchordata = getTestProgram().coder.types.encode(
                'PublicTransactionEvent',
                data,
            );
            expect(anchordata).toEqual(encoded);

            expect(deepEqual(decoded, data)).toBe(true);
        });
    });

    describe('invokeAccountsLayout', () => {
        const feePayer = new Keypair().publicKey;
        const authority = new Keypair().publicKey;
        const registeredProgramPda =
            defaultStaticAccountsStruct().registeredProgramPda;
        const noopProgram = defaultStaticAccountsStruct().noopProgram;
        const accountCompressionAuthority =
            defaultStaticAccountsStruct().accountCompressionAuthority;
        const accountCompressionProgram =
            defaultStaticAccountsStruct().accountCompressionProgram;

        const solPoolPda = LightSystemProgram.deriveCompressedSolPda();
        const decompressionRecipient =
            LightSystemProgram.deriveCompressedSolPda();
        const systemProgram = SystemProgram.programId;

        it('should return correct AccountMeta array with null solPoodPda', () => {
            const accounts = {
                feePayer,
                authority,
                registeredProgramPda,
                noopProgram,
                accountCompressionAuthority,
                accountCompressionProgram,
                solPoolPda: null,
                decompressionRecipient,
                systemProgram,
            };

            const expected = [
                { pubkey: feePayer, isSigner: true, isWritable: true },
                { pubkey: authority, isSigner: true, isWritable: false },
                {
                    pubkey: registeredProgramPda,
                    isSigner: false,
                    isWritable: false,
                },
                { pubkey: noopProgram, isSigner: false, isWritable: false },
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
                    pubkey: LightSystemProgram.programId,
                    isSigner: false,
                    isWritable: false,
                },
                {
                    pubkey: decompressionRecipient,
                    isSigner: false,
                    isWritable: true,
                },
                { pubkey: systemProgram, isSigner: false, isWritable: false },
            ];

            const result = invokeAccountsLayout(accounts);
            expect(result).toEqual(expected);
        });

        it('should return correct AccountMeta array with non-null solPoolPda', () => {
            const accounts = {
                feePayer,
                authority,
                registeredProgramPda,
                noopProgram,
                accountCompressionAuthority,
                accountCompressionProgram,
                solPoolPda,
                decompressionRecipient,
                systemProgram,
            };

            const expected = [
                { pubkey: feePayer, isSigner: true, isWritable: true },
                { pubkey: authority, isSigner: true, isWritable: false },
                {
                    pubkey: registeredProgramPda,
                    isSigner: false,
                    isWritable: false,
                },
                { pubkey: noopProgram, isSigner: false, isWritable: false },
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
                { pubkey: solPoolPda, isSigner: false, isWritable: true },
                {
                    pubkey: decompressionRecipient,
                    isSigner: false,
                    isWritable: true,
                },
                { pubkey: systemProgram, isSigner: false, isWritable: false },
            ];

            const result = invokeAccountsLayout(accounts);
            expect(result).toEqual(expected);
        });

        it('should return correct AccountMeta array with null decompressionRecipient', () => {
            const accounts = {
                feePayer,
                authority,
                registeredProgramPda,
                noopProgram,
                accountCompressionAuthority,
                accountCompressionProgram,
                solPoolPda,
                decompressionRecipient: null,
                systemProgram,
            };

            const expected = [
                { pubkey: feePayer, isSigner: true, isWritable: true },
                { pubkey: authority, isSigner: true, isWritable: false },
                {
                    pubkey: registeredProgramPda,
                    isSigner: false,
                    isWritable: false,
                },
                { pubkey: noopProgram, isSigner: false, isWritable: false },
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
                { pubkey: solPoolPda, isSigner: false, isWritable: true },
                {
                    pubkey: LightSystemProgram.programId,
                    isSigner: false,
                    isWritable: true,
                },
                { pubkey: systemProgram, isSigner: false, isWritable: false },
            ];

            const result = invokeAccountsLayout(accounts);
            expect(result).toEqual(expected);
        });
    });
});
