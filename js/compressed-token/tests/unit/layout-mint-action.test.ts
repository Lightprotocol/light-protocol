import { describe, it, expect } from 'vitest';
import { PublicKey, Keypair } from '@solana/web3.js';
import {
    encodeMintActionInstructionData,
    decodeMintActionInstructionData,
    MintActionCompressedInstructionData,
    Action,
    MINT_ACTION_DISCRIMINATOR,
} from '../../src/v3/layout/layout-mint-action';

describe('layout-mint-action', () => {
    describe('encodeMintActionInstructionData / decodeMintActionInstructionData', () => {
        it('should encode and decode basic instruction data without actions', () => {
            const mint = Keypair.generate().publicKey;

            const data: MintActionCompressedInstructionData = {
                leafIndex: 100,
                proveByIndex: true,
                rootIndex: 5,
                maxTopUp: 1000,
                createMint: null,
                actions: [],
                proof: null,
                cpiContext: null,
                mint: {
                    supply: 1000000n,
                    decimals: 9,
                    metadata: {
                        version: 1,
                        cmintDecompressed: true,
                        mint,
                        compressedAddress: Array(32).fill(1),
                    },
                    mintAuthority: mint,
                    freezeAuthority: null,
                    extensions: null,
                },
            };

            const encoded = encodeMintActionInstructionData(data);

            // Check discriminator
            expect(encoded.subarray(0, 1)).toEqual(MINT_ACTION_DISCRIMINATOR);

            const decoded = decodeMintActionInstructionData(encoded);

            expect(decoded.leafIndex).toBe(100);
            expect(decoded.proveByIndex).toBe(true);
            expect(decoded.rootIndex).toBe(5);
            expect(decoded.maxTopUp).toBe(1000);
            expect(decoded.actions.length).toBe(0);
            expect(decoded.mint.decimals).toBe(9);
        });

        it('should encode and decode with mintToCompressed action', () => {
            const mint = Keypair.generate().publicKey;
            const recipient1 = Keypair.generate().publicKey;
            const recipient2 = Keypair.generate().publicKey;

            const mintToCompressedAction: Action = {
                mintToCompressed: {
                    tokenAccountVersion: 1,
                    recipients: [
                        { recipient: recipient1, amount: 500n },
                        { recipient: recipient2, amount: 1500n },
                    ],
                },
            };

            const data: MintActionCompressedInstructionData = {
                leafIndex: 50,
                proveByIndex: false,
                rootIndex: 10,
                maxTopUp: 500,
                createMint: null,
                actions: [mintToCompressedAction],
                proof: null,
                cpiContext: null,
                mint: {
                    supply: 0n,
                    decimals: 6,
                    metadata: {
                        version: 1,
                        cmintDecompressed: false,
                        mint,
                        compressedAddress: Array(32).fill(2),
                    },
                    mintAuthority: mint,
                    freezeAuthority: null,
                    extensions: null,
                },
            };

            const encoded = encodeMintActionInstructionData(data);
            const decoded = decodeMintActionInstructionData(encoded);

            expect(decoded.actions.length).toBe(1);
            expect('mintToCompressed' in decoded.actions[0]).toBe(true);

            const action = decoded.actions[0] as {
                mintToCompressed: {
                    tokenAccountVersion: number;
                    recipients: { recipient: PublicKey; amount: bigint }[];
                };
            };
            expect(action.mintToCompressed.tokenAccountVersion).toBe(1);
            expect(action.mintToCompressed.recipients.length).toBe(2);
        });

        it('should encode and decode with mintToCToken action', () => {
            const mint = Keypair.generate().publicKey;

            const mintToCTokenAction: Action = {
                mintToCToken: {
                    accountIndex: 3,
                    amount: 1000000n,
                },
            };

            const data: MintActionCompressedInstructionData = {
                leafIndex: 0,
                proveByIndex: true,
                rootIndex: 0,
                maxTopUp: 0,
                createMint: null,
                actions: [mintToCTokenAction],
                proof: null,
                cpiContext: null,
                mint: {
                    supply: 1000000n,
                    decimals: 9,
                    metadata: {
                        version: 1,
                        cmintDecompressed: true,
                        mint,
                        compressedAddress: Array(32).fill(0),
                    },
                    mintAuthority: mint,
                    freezeAuthority: null,
                    extensions: null,
                },
            };

            const encoded = encodeMintActionInstructionData(data);
            const decoded = decodeMintActionInstructionData(encoded);

            expect(decoded.actions.length).toBe(1);
            expect('mintToCToken' in decoded.actions[0]).toBe(true);

            const action = decoded.actions[0] as {
                mintToCToken: { accountIndex: number; amount: bigint };
            };
            expect(action.mintToCToken.accountIndex).toBe(3);
        });

        it('should encode and decode with updateMintAuthority action', () => {
            const mint = Keypair.generate().publicKey;
            const newAuthority = Keypair.generate().publicKey;

            const updateAction: Action = {
                updateMintAuthority: {
                    newAuthority,
                },
            };

            const data: MintActionCompressedInstructionData = {
                leafIndex: 10,
                proveByIndex: true,
                rootIndex: 2,
                maxTopUp: 100,
                createMint: null,
                actions: [updateAction],
                proof: null,
                cpiContext: null,
                mint: {
                    supply: 500n,
                    decimals: 6,
                    metadata: {
                        version: 1,
                        cmintDecompressed: true,
                        mint,
                        compressedAddress: Array(32).fill(5),
                    },
                    mintAuthority: mint,
                    freezeAuthority: null,
                    extensions: null,
                },
            };

            const encoded = encodeMintActionInstructionData(data);
            const decoded = decodeMintActionInstructionData(encoded);

            expect(decoded.actions.length).toBe(1);
            expect('updateMintAuthority' in decoded.actions[0]).toBe(true);
        });

        it('should encode and decode with multiple actions', () => {
            const mint = Keypair.generate().publicKey;
            const recipient = Keypair.generate().publicKey;

            const actions: Action[] = [
                {
                    mintToCompressed: {
                        tokenAccountVersion: 1,
                        recipients: [{ recipient, amount: 1000n }],
                    },
                },
                {
                    updateMintAuthority: {
                        newAuthority: null,
                    },
                },
            ];

            const data: MintActionCompressedInstructionData = {
                leafIndex: 5,
                proveByIndex: true,
                rootIndex: 1,
                maxTopUp: 50,
                createMint: null,
                actions,
                proof: null,
                cpiContext: null,
                mint: {
                    supply: 1000n,
                    decimals: 9,
                    metadata: {
                        version: 1,
                        cmintDecompressed: true,
                        mint,
                        compressedAddress: Array(32).fill(7),
                    },
                    mintAuthority: mint,
                    freezeAuthority: null,
                    extensions: null,
                },
            };

            const encoded = encodeMintActionInstructionData(data);
            const decoded = decodeMintActionInstructionData(encoded);

            expect(decoded.actions.length).toBe(2);
        });

        it('should handle large supply values', () => {
            const mint = Keypair.generate().publicKey;
            const largeSupply = BigInt('18446744073709551615'); // max u64

            const data: MintActionCompressedInstructionData = {
                leafIndex: 0,
                proveByIndex: true,
                rootIndex: 0,
                maxTopUp: 0,
                createMint: null,
                actions: [],
                proof: null,
                cpiContext: null,
                mint: {
                    supply: largeSupply,
                    decimals: 9,
                    metadata: {
                        version: 1,
                        cmintDecompressed: true,
                        mint,
                        compressedAddress: Array(32).fill(0),
                    },
                    mintAuthority: mint,
                    freezeAuthority: null,
                    extensions: null,
                },
            };

            const encoded = encodeMintActionInstructionData(data);
            const decoded = decodeMintActionInstructionData(encoded);

            // BN returns bigint when converted, check as string to handle both types
            expect(decoded.mint.supply.toString()).toBe(largeSupply.toString());
        });

        it('should encode and decode with cpiContext', () => {
            const mint = Keypair.generate().publicKey;

            const data: MintActionCompressedInstructionData = {
                leafIndex: 0,
                proveByIndex: true,
                rootIndex: 0,
                maxTopUp: 0,
                createMint: null,
                actions: [],
                proof: null,
                cpiContext: {
                    setContext: true,
                    firstSetContext: true,
                    inTreeIndex: 1,
                    inQueueIndex: 2,
                    outQueueIndex: 3,
                    tokenOutQueueIndex: 4,
                    assignedAccountIndex: 5,
                    readOnlyAddressTrees: [6, 7, 8, 9],
                    addressTreePubkey: Array(32).fill(10),
                },
                mint: {
                    supply: 0n,
                    decimals: 9,
                    metadata: {
                        version: 1,
                        cmintDecompressed: true,
                        mint,
                        compressedAddress: Array(32).fill(0),
                    },
                    mintAuthority: mint,
                    freezeAuthority: null,
                    extensions: null,
                },
            };

            const encoded = encodeMintActionInstructionData(data);
            const decoded = decodeMintActionInstructionData(encoded);

            expect(decoded.cpiContext).not.toBe(null);
            expect(decoded.cpiContext?.setContext).toBe(true);
            expect(decoded.cpiContext?.firstSetContext).toBe(true);
            expect(decoded.cpiContext?.inTreeIndex).toBe(1);
        });
    });
});
