/**
 * Unit tests for all action builders in actions.ts.
 *
 * Tests cover: transfer, wrap/unwrap, compress/decompress, mint management,
 * ATA creation, and interface builders.
 */

import { describe, it, expect, vi } from 'vitest';
import { address } from '@solana/addresses';

import {
    // Transfer
    buildTransferDelegated,
    buildTransferInterface,

    // Wrap / Unwrap
    buildWrap,
    buildUnwrap,

    // Compress / Decompress
    buildCompress,
    buildDecompress,
    buildCompressSplTokenAccount,
    buildDecompressInterface,
    buildLoadAta,

    // Mint management
    buildCreateMint,
    buildUpdateMintAuthority,
    buildUpdateFreezeAuthority,
    buildUpdateMetadataField,
    buildUpdateMetadataAuthority,
    buildRemoveMetadataKey,
    buildDecompressMint,

    // Mint to
    buildMintToCompressed,
    buildMintToInterface,
    buildApproveAndMintTo,

    // ATA
    buildCreateAta,
    buildCreateAtaIdempotent,
    buildGetOrCreateAta,

    // Constants
    DISCRIMINATOR,
    SPL_TOKEN_PROGRAM_ID,
    LIGHT_TOKEN_PROGRAM_ID,
    LIGHT_TOKEN_CONFIG,
    LIGHT_TOKEN_RENT_SPONSOR,

    IndexerError,
    IndexerErrorCode,
} from '../../src/index.js';

import {
    createMockTokenAccount,
    createMockIndexer,
    createMockRpc,
    createMockRpcWithMint,
    createMockMintContext,
    createMockSplInterfaceInfo,
    createBase64MintData,
    createMockAccountWithHash,
    createTransferMockIndexer,
    MOCK_OWNER,
    MOCK_MINT,
    MOCK_POOL,
    MOCK_TREE,
    MOCK_QUEUE,
    MOCK_MINT_SIGNER,
} from './helpers.js';

const FEE_PAYER = address('BPFLoaderUpgradeab1e11111111111111111111111');
const RECIPIENT = address('GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy');
const DELEGATE = address('Sysvar1111111111111111111111111111111111111');

// ============================================================================
// TRANSFER BUILDERS
// ============================================================================

describe('buildTransferDelegated', () => {
    it('builds instruction with delegate in packed accounts', async () => {
        const accounts = [createMockAccountWithHash(1000n, 0xab, 5, DELEGATE)];
        const indexer = createTransferMockIndexer(accounts, [
            { hashByte: 0xab, rootIndex: 10 },
        ]);

        const result = await buildTransferDelegated({
            indexer,
            delegate: DELEGATE,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            amount: 500n,
            recipientOwner: RECIPIENT,
            feePayer: FEE_PAYER,
        });

        expect(result.instruction.data[0]).toBe(DISCRIMINATOR.TRANSFER2);
        expect(result.totalInputAmount).toBe(1000n);
        // Delegate should appear in packed accounts
        expect(
            result.instruction.accounts.some(
                (acc) => acc.address === DELEGATE,
            ),
        ).toBe(true);
    });

    it('throws when no accounts found', async () => {
        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: [], cursor: null },
            }),
        });

        await expect(
            buildTransferDelegated({
                indexer,
                delegate: DELEGATE,
                owner: MOCK_OWNER,
                mint: MOCK_MINT,
                amount: 100n,
                recipientOwner: RECIPIENT,
                feePayer: FEE_PAYER,
            }),
        ).rejects.toThrow(IndexerError);
    });
});

describe('buildTransferInterface', () => {
    it('returns instructions array wrapping transfer result', async () => {
        const accounts = [createMockAccountWithHash(1000n, 0xab, 5)];
        const indexer = createTransferMockIndexer(accounts, [
            { hashByte: 0xab, rootIndex: 10 },
        ]);

        const result = await buildTransferInterface({
            indexer,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            amount: 500n,
            recipientOwner: RECIPIENT,
            feePayer: FEE_PAYER,
        });

        expect(result.instructions).toHaveLength(1);
        expect(result.instructions[0].data[0]).toBe(DISCRIMINATOR.TRANSFER2);
        expect(result.transferResult.totalInputAmount).toBe(1000n);
    });
});

// ============================================================================
// WRAP / UNWRAP BUILDERS
// ============================================================================

describe('buildWrap', () => {
    it('builds wrap instruction with explicit decimals', async () => {
        const rpc = createMockRpc();
        const splInfo = createMockSplInterfaceInfo();

        const ix = await buildWrap({
            rpc,
            source: MOCK_OWNER,
            destination: RECIPIENT,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            amount: 1000n,
            decimals: 9,
            tokenProgram: SPL_TOKEN_PROGRAM_ID,
            splInterfaceInfo: splInfo,
        });

        expect(ix.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
        expect(ix.accounts.length).toBeGreaterThan(0);
    });

    it('auto-fetches decimals when omitted', async () => {
        const rpc = createMockRpcWithMint(6);
        const splInfo = createMockSplInterfaceInfo();

        const ix = await buildWrap({
            rpc,
            source: MOCK_OWNER,
            destination: RECIPIENT,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            amount: 1000n,
            splInterfaceInfo: splInfo,
        });

        expect(ix.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
    });

    it('defaults tokenProgram to SPL_TOKEN_PROGRAM_ID', async () => {
        const rpc = createMockRpcWithMint(9);
        const splInfo = createMockSplInterfaceInfo();

        const ix = await buildWrap({
            rpc,
            source: MOCK_OWNER,
            destination: RECIPIENT,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            amount: 1000n,
            splInterfaceInfo: splInfo,
        });

        expect(ix.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
    });
});

describe('buildUnwrap', () => {
    it('builds unwrap instruction with explicit decimals', async () => {
        const rpc = createMockRpc();
        const splInfo = createMockSplInterfaceInfo();

        const ix = await buildUnwrap({
            rpc,
            source: MOCK_OWNER,
            destination: RECIPIENT,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            amount: 1000n,
            decimals: 9,
            tokenProgram: SPL_TOKEN_PROGRAM_ID,
            splInterfaceInfo: splInfo,
        });

        expect(ix.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
    });

    it('auto-fetches decimals when omitted', async () => {
        const rpc = createMockRpcWithMint(6);
        const splInfo = createMockSplInterfaceInfo();

        const ix = await buildUnwrap({
            rpc,
            source: MOCK_OWNER,
            destination: RECIPIENT,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            amount: 1000n,
            splInterfaceInfo: splInfo,
        });

        expect(ix.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
    });
});

// ============================================================================
// COMPRESS / DECOMPRESS BUILDERS
// ============================================================================

describe('buildCompress', () => {
    it('builds Transfer2 instruction with compression struct', async () => {
        const rpc = createMockRpc();
        const splInfo = createMockSplInterfaceInfo();

        const ix = await buildCompress({
            rpc,
            source: MOCK_OWNER,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            amount: 1000n,
            decimals: 9,
            tokenProgram: SPL_TOKEN_PROGRAM_ID,
            outputQueue: MOCK_QUEUE,
            splInterfaceInfo: splInfo,
        });

        expect(ix.data[0]).toBe(DISCRIMINATOR.TRANSFER2);
        expect(ix.accounts.length).toBeGreaterThan(0);
    });

    it('auto-fetches decimals when omitted', async () => {
        const rpc = createMockRpcWithMint(6);
        const splInfo = createMockSplInterfaceInfo();

        const ix = await buildCompress({
            rpc,
            source: MOCK_OWNER,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            amount: 1000n,
            outputQueue: MOCK_QUEUE,
            splInterfaceInfo: splInfo,
        });

        expect(ix.data[0]).toBe(DISCRIMINATOR.TRANSFER2);
    });
});

describe('buildDecompress', () => {
    it('builds Transfer2 instruction with decompress compression', async () => {
        const accounts = [createMockAccountWithHash(1000n, 0xab, 5)];
        const indexer = createTransferMockIndexer(accounts, [
            { hashByte: 0xab, rootIndex: 10 },
        ]);
        const rpc = createMockRpc();
        const splInfo = createMockSplInterfaceInfo();

        const result = await buildDecompress({
            rpc,
            indexer,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            amount: 500n,
            destination: RECIPIENT,
            decimals: 9,
            tokenProgram: SPL_TOKEN_PROGRAM_ID,
            splInterfaceInfo: splInfo,
        });

        expect(result.instruction.data[0]).toBe(DISCRIMINATOR.TRANSFER2);
        expect(result.totalInputAmount).toBe(1000n);
    });

    it('creates change output when input > amount', async () => {
        const accounts = [createMockAccountWithHash(1000n, 0xab, 5)];
        const indexer = createTransferMockIndexer(accounts, [
            { hashByte: 0xab, rootIndex: 10 },
        ]);
        const rpc = createMockRpc();
        const splInfo = createMockSplInterfaceInfo();

        const result = await buildDecompress({
            rpc,
            indexer,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            amount: 300n,
            destination: RECIPIENT,
            decimals: 9,
            tokenProgram: SPL_TOKEN_PROGRAM_ID,
            splInterfaceInfo: splInfo,
        });

        expect(result.totalInputAmount).toBeGreaterThan(300n);
    });
});

describe('buildCompressSplTokenAccount', () => {
    it('delegates to buildCompress', async () => {
        const rpc = createMockRpc();
        const splInfo = createMockSplInterfaceInfo();

        const ix = await buildCompressSplTokenAccount({
            rpc,
            source: MOCK_OWNER,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            amount: 1000n,
            decimals: 9,
            tokenProgram: SPL_TOKEN_PROGRAM_ID,
            outputQueue: MOCK_QUEUE,
            splInterfaceInfo: splInfo,
        });

        expect(ix.data[0]).toBe(DISCRIMINATOR.TRANSFER2);
    });
});

describe('buildDecompressInterface', () => {
    it('uses explicit destination without creating ATA', async () => {
        const accounts = [createMockAccountWithHash(1000n, 0xab, 5)];
        const indexer = createTransferMockIndexer(accounts, [
            { hashByte: 0xab, rootIndex: 10 },
        ]);
        const rpc = createMockRpc();
        const splInfo = createMockSplInterfaceInfo();

        const result = await buildDecompressInterface({
            rpc,
            indexer,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            amount: 500n,
            destination: RECIPIENT,
            decimals: 9,
            tokenProgram: SPL_TOKEN_PROGRAM_ID,
            splInterfaceInfo: splInfo,
        });

        expect(result.destination).toBe(RECIPIENT);
        // Only decompress instruction, no createAta
        expect(result.instructions).toHaveLength(1);
        expect(result.instructions[0].data[0]).toBe(DISCRIMINATOR.TRANSFER2);
    });

    it('returns empty instructions when amount is 0', async () => {
        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: [], cursor: null },
            }),
        });
        const rpc = createMockRpc();

        const result = await buildDecompressInterface({
            rpc,
            indexer,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            destination: RECIPIENT,
        });

        // No cold balance = no decompress instruction
        expect(result.instructions).toHaveLength(0);
    });
});

describe('buildLoadAta', () => {
    it('returns empty array when no cold accounts', async () => {
        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: [], cursor: null },
            }),
        });
        const rpc = createMockRpc();

        const result = await buildLoadAta({
            rpc,
            indexer,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            destination: RECIPIENT,
            decimals: 9,
            tokenProgram: SPL_TOKEN_PROGRAM_ID,
        });

        expect(result).toEqual([]);
    });

    it('returns decompress instruction when cold balance exists', async () => {
        const accounts = [createMockAccountWithHash(500n, 0xab, 5)];
        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: accounts, cursor: null },
            }),
            getValidityProof: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: {
                    proof: { a: new Uint8Array(32), b: new Uint8Array(64), c: new Uint8Array(32) },
                    accounts: [{ hash: new Uint8Array(32).fill(0xab), root: new Uint8Array(32), rootIndex: { rootIndex: 1, proveByIndex: false }, leafIndex: 5, treeInfo: { tree: MOCK_TREE, queue: MOCK_QUEUE, treeType: 2 } }],
                    addresses: [],
                },
            }),
        });
        const rpc = createMockRpc();
        const splInfo = createMockSplInterfaceInfo();

        const result = await buildLoadAta({
            rpc,
            indexer,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            destination: RECIPIENT,
            decimals: 9,
            tokenProgram: SPL_TOKEN_PROGRAM_ID,
            splInterfaceInfo: splInfo,
        });

        expect(result).toHaveLength(1);
        expect(result[0].data[0]).toBe(DISCRIMINATOR.TRANSFER2);
    });
});

// ============================================================================
// MINT MANAGEMENT BUILDERS
// ============================================================================

describe('buildCreateMint', () => {
    it('builds MintAction instruction with discriminator 103', async () => {
        const ix = await buildCreateMint({
            mintSigner: MOCK_MINT_SIGNER,
            authority: MOCK_OWNER,
            feePayer: FEE_PAYER,
            outOutputQueue: MOCK_QUEUE,
            merkleTree: MOCK_TREE,
            decimals: 9,
            mintAuthority: MOCK_OWNER,
        });

        expect(ix.data[0]).toBe(DISCRIMINATOR.MINT_ACTION);
        expect(ix.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
    });

    it('converts Address types to bytes for authorities', async () => {
        const ix = await buildCreateMint({
            mintSigner: MOCK_MINT_SIGNER,
            authority: MOCK_OWNER,
            feePayer: FEE_PAYER,
            outOutputQueue: MOCK_QUEUE,
            merkleTree: MOCK_TREE,
            decimals: 6,
            mintAuthority: MOCK_OWNER,
            freezeAuthority: RECIPIENT,
        });

        expect(ix.data[0]).toBe(DISCRIMINATOR.MINT_ACTION);
    });

    it('handles null freezeAuthority', async () => {
        const ix = await buildCreateMint({
            mintSigner: MOCK_MINT_SIGNER,
            authority: MOCK_OWNER,
            feePayer: FEE_PAYER,
            outOutputQueue: MOCK_QUEUE,
            merkleTree: MOCK_TREE,
            decimals: 9,
            mintAuthority: MOCK_OWNER,
            freezeAuthority: null,
        });

        expect(ix.data[0]).toBe(DISCRIMINATOR.MINT_ACTION);
    });
});

describe('buildUpdateMintAuthority', () => {
    it('builds instruction with mintContext override', async () => {
        const ctx = createMockMintContext();
        const indexer = createMockIndexer();

        const ix = await buildUpdateMintAuthority({
            indexer,
            mint: MOCK_MINT,
            authority: MOCK_OWNER,
            feePayer: FEE_PAYER,
            newAuthority: RECIPIENT,
            mintContext: ctx,
        });

        expect(ix.data[0]).toBe(DISCRIMINATOR.MINT_ACTION);
    });

    it('handles null newAuthority (revoke)', async () => {
        const ctx = createMockMintContext();
        const indexer = createMockIndexer();

        const ix = await buildUpdateMintAuthority({
            indexer,
            mint: MOCK_MINT,
            authority: MOCK_OWNER,
            feePayer: FEE_PAYER,
            newAuthority: null,
            mintContext: ctx,
        });

        expect(ix.data[0]).toBe(DISCRIMINATOR.MINT_ACTION);
    });
});

describe('buildUpdateFreezeAuthority', () => {
    it('builds instruction with mintContext', async () => {
        const ctx = createMockMintContext();
        const indexer = createMockIndexer();

        const ix = await buildUpdateFreezeAuthority({
            indexer,
            mint: MOCK_MINT,
            authority: MOCK_OWNER,
            feePayer: FEE_PAYER,
            newAuthority: RECIPIENT,
            mintContext: ctx,
        });

        expect(ix.data[0]).toBe(DISCRIMINATOR.MINT_ACTION);
    });

    it('handles null newAuthority (revoke)', async () => {
        const ctx = createMockMintContext();
        const indexer = createMockIndexer();

        const ix = await buildUpdateFreezeAuthority({
            indexer,
            mint: MOCK_MINT,
            authority: MOCK_OWNER,
            feePayer: FEE_PAYER,
            newAuthority: null,
            mintContext: ctx,
        });

        expect(ix.data[0]).toBe(DISCRIMINATOR.MINT_ACTION);
    });
});

describe('buildUpdateMetadataField', () => {
    const ctx = createMockMintContext();

    it.each([
        ['name', 'TestToken'],
        ['symbol', 'TT'],
        ['uri', 'https://example.com'],
    ] as const)('builds instruction for fieldType=%s', async (fieldType, value) => {
        const indexer = createMockIndexer();

        const ix = await buildUpdateMetadataField({
            indexer,
            mint: MOCK_MINT,
            authority: MOCK_OWNER,
            feePayer: FEE_PAYER,
            fieldType,
            value,
            mintContext: ctx,
        });

        expect(ix.data[0]).toBe(DISCRIMINATOR.MINT_ACTION);
    });

    it('encodes custom key for fieldType=custom', async () => {
        const indexer = createMockIndexer();

        const ix = await buildUpdateMetadataField({
            indexer,
            mint: MOCK_MINT,
            authority: MOCK_OWNER,
            feePayer: FEE_PAYER,
            fieldType: 'custom',
            value: 'myValue',
            customKey: 'myKey',
            mintContext: ctx,
        });

        expect(ix.data[0]).toBe(DISCRIMINATOR.MINT_ACTION);
    });
});

describe('buildUpdateMetadataAuthority', () => {
    it('builds instruction with default extensionIndex', async () => {
        const ctx = createMockMintContext({ metadataExtensionIndex: 2 });
        const indexer = createMockIndexer();

        const ix = await buildUpdateMetadataAuthority({
            indexer,
            mint: MOCK_MINT,
            authority: MOCK_OWNER,
            feePayer: FEE_PAYER,
            newAuthority: RECIPIENT,
            mintContext: ctx,
        });

        expect(ix.data[0]).toBe(DISCRIMINATOR.MINT_ACTION);
    });

    it('uses explicit extensionIndex when provided', async () => {
        const ctx = createMockMintContext();
        const indexer = createMockIndexer();

        const ix = await buildUpdateMetadataAuthority({
            indexer,
            mint: MOCK_MINT,
            authority: MOCK_OWNER,
            feePayer: FEE_PAYER,
            newAuthority: RECIPIENT,
            extensionIndex: 5,
            mintContext: ctx,
        });

        expect(ix.data[0]).toBe(DISCRIMINATOR.MINT_ACTION);
    });
});

describe('buildRemoveMetadataKey', () => {
    it('builds instruction with idempotent=false', async () => {
        const ctx = createMockMintContext();
        const indexer = createMockIndexer();

        const ix = await buildRemoveMetadataKey({
            indexer,
            mint: MOCK_MINT,
            authority: MOCK_OWNER,
            feePayer: FEE_PAYER,
            key: 'website',
            mintContext: ctx,
        });

        expect(ix.data[0]).toBe(DISCRIMINATOR.MINT_ACTION);
    });

    it('builds instruction with idempotent=true', async () => {
        const ctx = createMockMintContext();
        const indexer = createMockIndexer();

        const ix = await buildRemoveMetadataKey({
            indexer,
            mint: MOCK_MINT,
            authority: MOCK_OWNER,
            feePayer: FEE_PAYER,
            key: 'website',
            idempotent: true,
            mintContext: ctx,
        });

        expect(ix.data[0]).toBe(DISCRIMINATOR.MINT_ACTION);
    });
});

// ============================================================================
// MINT TO BUILDERS
// ============================================================================

describe('buildMintToCompressed', () => {
    it('builds instruction with multiple recipients', async () => {
        const ctx = createMockMintContext();
        const indexer = createMockIndexer();

        const ix = await buildMintToCompressed({
            indexer,
            mint: MOCK_MINT,
            authority: MOCK_OWNER,
            feePayer: FEE_PAYER,
            recipients: [
                { recipient: RECIPIENT, amount: 1000n },
                { recipient: MOCK_OWNER, amount: 2000n },
            ],
            mintContext: ctx,
        });

        expect(ix.data[0]).toBe(DISCRIMINATOR.MINT_ACTION);
    });

    it('builds instruction with single recipient', async () => {
        const ctx = createMockMintContext();
        const indexer = createMockIndexer();

        const ix = await buildMintToCompressed({
            indexer,
            mint: MOCK_MINT,
            authority: MOCK_OWNER,
            feePayer: FEE_PAYER,
            recipients: [{ recipient: RECIPIENT, amount: 500n }],
            mintContext: ctx,
        });

        expect(ix.data[0]).toBe(DISCRIMINATOR.MINT_ACTION);
    });
});

describe('buildMintToInterface', () => {
    it('includes tokenAccount in packed accounts as writable', async () => {
        const ctx = createMockMintContext();
        const indexer = createMockIndexer();
        const tokenAccount = address('Vote111111111111111111111111111111111111111');

        const ix = await buildMintToInterface({
            indexer,
            mint: MOCK_MINT,
            authority: MOCK_OWNER,
            feePayer: FEE_PAYER,
            tokenAccount,
            amount: 1000n,
            mintContext: ctx,
        });

        expect(ix.data[0]).toBe(DISCRIMINATOR.MINT_ACTION);
        // tokenAccount should be in the remaining accounts
        expect(
            ix.accounts.some((acc) => acc.address === tokenAccount),
        ).toBe(true);
    });
});

describe('buildDecompressMint', () => {
    it('uses default rentPayment and writeTopUp', async () => {
        const ctx = createMockMintContext();
        const indexer = createMockIndexer();

        const ix = await buildDecompressMint({
            indexer,
            mint: MOCK_MINT,
            authority: MOCK_OWNER,
            feePayer: FEE_PAYER,
            mintContext: ctx,
        });

        expect(ix.data[0]).toBe(DISCRIMINATOR.MINT_ACTION);
        // Verify config accounts are present
        expect(
            ix.accounts.some((acc) => acc.address === LIGHT_TOKEN_CONFIG),
        ).toBe(true);
        expect(
            ix.accounts.some((acc) => acc.address === LIGHT_TOKEN_RENT_SPONSOR),
        ).toBe(true);
    });

    it('accepts custom config addresses', async () => {
        const ctx = createMockMintContext();
        const indexer = createMockIndexer();
        const customConfig = address('Vote111111111111111111111111111111111111111');
        const customSponsor = address('11111111111111111111111111111111');

        const ix = await buildDecompressMint({
            indexer,
            mint: MOCK_MINT,
            authority: MOCK_OWNER,
            feePayer: FEE_PAYER,
            compressibleConfig: customConfig,
            rentSponsor: customSponsor,
            mintContext: ctx,
        });

        expect(ix.data[0]).toBe(DISCRIMINATOR.MINT_ACTION);
        expect(
            ix.accounts.some((acc) => acc.address === customConfig),
        ).toBe(true);
        expect(
            ix.accounts.some((acc) => acc.address === customSponsor),
        ).toBe(true);
    });
});

// ============================================================================
// APPROVE AND MINT TO
// ============================================================================

describe('buildApproveAndMintTo', () => {
    it('returns two instructions [approve, mintTo]', () => {
        const result = buildApproveAndMintTo({
            tokenAccount: RECIPIENT,
            mint: MOCK_MINT,
            delegate: DELEGATE,
            owner: MOCK_OWNER,
            mintAuthority: MOCK_OWNER,
            approveAmount: 1000n,
            mintAmount: 500n,
        });

        expect(result).toHaveLength(2);
        expect(result[0].data[0]).toBe(DISCRIMINATOR.APPROVE);
        expect(result[1].data[0]).toBe(DISCRIMINATOR.MINT_TO);
    });

    it('passes maxTopUp to both instructions', () => {
        const result = buildApproveAndMintTo({
            tokenAccount: RECIPIENT,
            mint: MOCK_MINT,
            delegate: DELEGATE,
            owner: MOCK_OWNER,
            mintAuthority: MOCK_OWNER,
            approveAmount: 1000n,
            mintAmount: 500n,
            maxTopUp: 100,
        });

        expect(result).toHaveLength(2);
    });
});

// ============================================================================
// ATA BUILDERS
// ============================================================================

describe('buildCreateAta', () => {
    it('derives ATA and returns instruction + address + bump', async () => {
        const result = await buildCreateAta({
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            feePayer: FEE_PAYER,
        });

        expect(result.instruction).toBeDefined();
        expect(result.ata).toBeDefined();
        expect(typeof result.bump).toBe('number');
        expect(result.bump).toBeGreaterThanOrEqual(0);
        expect(result.bump).toBeLessThanOrEqual(255);
        expect(result.instruction.data[0]).toBe(DISCRIMINATOR.CREATE_ATA);
    });

    it('produces consistent ATA address', async () => {
        const result1 = await buildCreateAta({
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            feePayer: FEE_PAYER,
        });
        const result2 = await buildCreateAta({
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            feePayer: FEE_PAYER,
        });

        expect(result1.ata).toBe(result2.ata);
        expect(result1.bump).toBe(result2.bump);
    });
});

describe('buildCreateAtaIdempotent', () => {
    it('uses idempotent discriminator', async () => {
        const result = await buildCreateAtaIdempotent({
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            feePayer: FEE_PAYER,
        });

        expect(result.instruction).toBeDefined();
        expect(result.ata).toBeDefined();
        expect(result.instruction.data[0]).toBe(
            DISCRIMINATOR.CREATE_ATA_IDEMPOTENT,
        );
    });
});

describe('buildGetOrCreateAta', () => {
    it('returns create + decompress instructions when ATA missing and cold balance exists', async () => {
        const accounts = [createMockAccountWithHash(500n, 0xab, 5)];
        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: accounts, cursor: null },
            }),
            getValidityProof: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: {
                    proof: { a: new Uint8Array(32), b: new Uint8Array(64), c: new Uint8Array(32) },
                    accounts: [{
                        hash: new Uint8Array(32).fill(0xab),
                        root: new Uint8Array(32),
                        rootIndex: { rootIndex: 1, proveByIndex: false },
                        leafIndex: 5,
                        treeInfo: { tree: MOCK_TREE, queue: MOCK_QUEUE, treeType: 2 },
                    }],
                    addresses: [],
                },
            }),
        });

        // RPC returns null = ATA doesn't exist
        const rpc = createMockRpc();
        const splInfo = createMockSplInterfaceInfo();

        const result = await buildGetOrCreateAta({
            rpc,
            indexer,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            feePayer: FEE_PAYER,
            decimals: 9,
            tokenProgram: SPL_TOKEN_PROGRAM_ID,
            splInterfaceInfo: splInfo,
        });

        expect(result.ata).toBeDefined();
        // Should have create ATA + decompress
        expect(result.instructions.length).toBeGreaterThanOrEqual(2);
        expect(result.coldBalance).toBe(500n);
        expect(result.hotBalance).toBe(0n);
    });

    it('skips create instruction when ATA exists', async () => {
        // Build a 72-byte account with balance=1000 at offset 64
        const accountBytes = new Uint8Array(72);
        const view = new DataView(accountBytes.buffer);
        view.setBigUint64(64, 1000n, true);
        const base64 = btoa(String.fromCharCode(...accountBytes));

        const rpc = createMockRpc({
            getAccountInfo: vi.fn().mockResolvedValue({
                value: {
                    owner: LIGHT_TOKEN_PROGRAM_ID,
                    data: [base64, 'base64'],
                },
            }),
        });

        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: [], cursor: null },
            }),
        });

        const result = await buildGetOrCreateAta({
            rpc,
            indexer,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            feePayer: FEE_PAYER,
        });

        // ATA exists with no cold balance = no instructions
        expect(result.instructions).toHaveLength(0);
        expect(result.hotBalance).toBe(1000n);
        expect(result.coldBalance).toBe(0n);
        expect(result.totalBalance).toBe(1000n);
    });

    it('returns only create instruction when ATA missing and no cold balance', async () => {
        const rpc = createMockRpc(); // returns null = no ATA

        const indexer = createMockIndexer({
            getCompressedTokenAccountsByOwner: vi.fn().mockResolvedValue({
                context: { slot: 100n },
                value: { items: [], cursor: null },
            }),
        });

        const result = await buildGetOrCreateAta({
            rpc,
            indexer,
            owner: MOCK_OWNER,
            mint: MOCK_MINT,
            feePayer: FEE_PAYER,
        });

        // Just create ATA, no decompress
        expect(result.instructions).toHaveLength(1);
        expect(result.coldBalance).toBe(0n);
        expect(result.hotBalance).toBe(0n);
    });
});
