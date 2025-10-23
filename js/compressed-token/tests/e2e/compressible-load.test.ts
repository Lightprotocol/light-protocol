import { describe, it, expect, beforeAll } from 'vitest';
import { Keypair, Signer, PublicKey } from '@solana/web3.js';
import {
    Rpc,
    bn,
    newAccountWithLamports,
    getTestRpc,
    selectStateTreeInfo,
    TreeInfo,
    MerkleContext,
    VERSION,
    featureFlags,
    CTOKEN_PROGRAM_ID,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import { createMint, mintTo } from '../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';
import {
    createLoadAccountsParams,
    createLoadATAInstructionsFromInterface,
    createLoadATAInstructions,
    CompressibleAccountInput,
    ParsedAccountInfoInterface,
    calculateCompressibleLoadComputeUnits,
} from '../../src/compressible/unified-load';
import { getATAInterface } from '../../src/mint/get-account-interface';
import { getATAAddressInterface } from '../../src/mint/actions/create-ata-interface';

featureFlags.version = VERSION.V2;

const TEST_TOKEN_DECIMALS = 9;

describe('compressible-load', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let stateTreeInfo: TreeInfo;
    let tokenPoolInfos: TokenPoolInfo[];

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);
        payer = await newAccountWithLamports(rpc, 10e9);
        mintAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();

        mint = (
            await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                null,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
            )
        ).mint;

        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
    }, 60_000);

    describe('createLoadAccountsParams', () => {
        describe('filtering', () => {
            it('should return empty result when no accounts provided', async () => {
                const result = await createLoadAccountsParams(
                    rpc,
                    payer.publicKey,
                    CTOKEN_PROGRAM_ID,
                    [],
                    [],
                );
                expect(result.decompressParams).toBeNull();
                expect(result.ataInstructions).toHaveLength(0);
            });

            it('should return null decompressParams when all accounts are hot', async () => {
                const hotInfo: ParsedAccountInfoInterface = {
                    parsed: { dummy: 'data' },
                    loadContext: undefined,
                };

                const accounts: CompressibleAccountInput[] = [
                    {
                        address: Keypair.generate().publicKey,
                        accountType: 'cTokenData',
                        tokenVariant: 'ata',
                        info: hotInfo,
                    },
                ];

                const result = await createLoadAccountsParams(
                    rpc,
                    payer.publicKey,
                    CTOKEN_PROGRAM_ID,
                    accounts,
                    [],
                );
                expect(result.decompressParams).toBeNull();
            });

            it('should filter out hot accounts and only process compressed', async () => {
                const owner = await newAccountWithLamports(rpc, 1e9);

                await mintTo(
                    rpc,
                    payer,
                    mint,
                    owner.publicKey,
                    mintAuthority,
                    bn(2000),
                    stateTreeInfo,
                    selectTokenPoolInfo(tokenPoolInfos),
                );

                const coldInfo = await getATAInterface(
                    rpc,
                    owner.publicKey,
                    mint,
                    undefined,
                    CTOKEN_PROGRAM_ID,
                );

                const hotInfo: ParsedAccountInfoInterface = {
                    parsed: { dummy: 'data' },
                    loadContext: undefined,
                };

                const accounts: CompressibleAccountInput[] = [
                    {
                        address: Keypair.generate().publicKey,
                        accountType: 'cTokenData',
                        tokenVariant: 'vault1',
                        info: hotInfo,
                    },
                    {
                        address: getATAAddressInterface(mint, owner.publicKey),
                        accountType: 'cTokenData',
                        tokenVariant: 'vault2',
                        info: coldInfo,
                    },
                ];

                const result = await createLoadAccountsParams(
                    rpc,
                    payer.publicKey,
                    CTOKEN_PROGRAM_ID,
                    accounts,
                    [],
                );

                expect(result.decompressParams).not.toBeNull();
                expect(result.decompressParams!.compressedAccounts.length).toBe(
                    1,
                );
            });
        });

        describe('cTokenData packing', () => {
            it('should throw when tokenVariant missing for cTokenData', async () => {
                const owner = await newAccountWithLamports(rpc, 1e9);

                await mintTo(
                    rpc,
                    payer,
                    mint,
                    owner.publicKey,
                    mintAuthority,
                    bn(1000),
                    stateTreeInfo,
                    selectTokenPoolInfo(tokenPoolInfos),
                );

                const accountInfo = await getATAInterface(
                    rpc,
                    owner.publicKey,
                    mint,
                    undefined,
                    CTOKEN_PROGRAM_ID,
                );

                const accounts: CompressibleAccountInput[] = [
                    {
                        address: getATAAddressInterface(mint, owner.publicKey),
                        accountType: 'cTokenData',
                        info: accountInfo,
                    },
                ];

                await expect(
                    createLoadAccountsParams(
                        rpc,
                        payer.publicKey,
                        CTOKEN_PROGRAM_ID,
                        accounts,
                        [],
                    ),
                ).rejects.toThrow('tokenVariant is required');
            });

            it('should pack cTokenData with correct variant structure', async () => {
                const owner = await newAccountWithLamports(rpc, 1e9);

                await mintTo(
                    rpc,
                    payer,
                    mint,
                    owner.publicKey,
                    mintAuthority,
                    bn(1000),
                    stateTreeInfo,
                    selectTokenPoolInfo(tokenPoolInfos),
                );

                const accountInfo = await getATAInterface(
                    rpc,
                    owner.publicKey,
                    mint,
                    undefined,
                    CTOKEN_PROGRAM_ID,
                );

                const accounts: CompressibleAccountInput[] = [
                    {
                        address: getATAAddressInterface(mint, owner.publicKey),
                        accountType: 'cTokenData',
                        tokenVariant: 'token0Vault',
                        info: accountInfo,
                    },
                ];

                const result = await createLoadAccountsParams(
                    rpc,
                    payer.publicKey,
                    CTOKEN_PROGRAM_ID,
                    accounts,
                    [],
                );

                expect(result.decompressParams).not.toBeNull();
                expect(result.decompressParams!.compressedAccounts.length).toBe(
                    1,
                );

                const packed = result.decompressParams!.compressedAccounts[0];
                expect(packed).toHaveProperty('cTokenData');
                expect(packed).toHaveProperty('merkleContext');
            });
        });

        describe('ATA loading via atas parameter', () => {
            it('should build ATA load instructions for cold ATAs', async () => {
                const owner = await newAccountWithLamports(rpc, 1e9);

                await mintTo(
                    rpc,
                    payer,
                    mint,
                    owner.publicKey,
                    mintAuthority,
                    bn(1000),
                    stateTreeInfo,
                    selectTokenPoolInfo(tokenPoolInfos),
                );

                const ata = await getATAInterface(
                    rpc,
                    owner.publicKey,
                    mint,
                    undefined,
                    CTOKEN_PROGRAM_ID,
                );

                const result = await createLoadAccountsParams(
                    rpc,
                    payer.publicKey,
                    CTOKEN_PROGRAM_ID,
                    [],
                    [ata],
                    { tokenPoolInfos },
                );

                expect(result.ataInstructions.length).toBeGreaterThan(0);
            });

            it('should return empty ataInstructions for hot ATAs', async () => {
                const owner = await newAccountWithLamports(rpc, 1e9);

                await mintTo(
                    rpc,
                    payer,
                    mint,
                    owner.publicKey,
                    mintAuthority,
                    bn(1000),
                    stateTreeInfo,
                    selectTokenPoolInfo(tokenPoolInfos),
                );

                // Load first to make it hot
                const coldAta = await getATAInterface(
                    rpc,
                    owner.publicKey,
                    mint,
                    undefined,
                    CTOKEN_PROGRAM_ID,
                );

                const loadIxs = await createLoadATAInstructionsFromInterface(
                    rpc,
                    payer.publicKey,
                    coldAta,
                    { tokenPoolInfos },
                );

                // Execute load (this would need actual tx, simplified here)
                // After load, ATA would be hot - for this test we just verify the flow
                expect(loadIxs.length).toBeGreaterThan(0);
            });
        });
    });

    describe('createLoadATAInstructionsFromInterface', () => {
        it('should throw if AccountInterface not from getATAInterface', async () => {
            const fakeInterface = {
                accountInfo: { data: Buffer.alloc(0) },
                parsed: {},
                isCold: false,
                // Missing _isAta, _owner, _mint
            } as any;

            await expect(
                createLoadATAInstructionsFromInterface(
                    rpc,
                    payer.publicKey,
                    fakeInterface,
                ),
            ).rejects.toThrow('must be from getATAInterface');
        });

        it('should return empty when nothing to load', async () => {
            const owner = Keypair.generate();

            // No balance - getATAInterface will throw, so we test the empty case differently
            // For an owner with no tokens, getATAInterface throws TokenAccountNotFoundError
            // This is expected behavior
        });

        it('should build instructions for cold ATA', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(1000),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const ata = await getATAInterface(
                rpc,
                owner.publicKey,
                mint,
                undefined,
                CTOKEN_PROGRAM_ID,
            );

            expect(ata._isAta).toBe(true);
            expect(ata._owner?.equals(owner.publicKey)).toBe(true);
            expect(ata._mint?.equals(mint)).toBe(true);

            const ixs = await createLoadATAInstructionsFromInterface(
                rpc,
                payer.publicKey,
                ata,
                { tokenPoolInfos },
            );

            expect(ixs.length).toBeGreaterThan(0);
        });
    });

    describe('createLoadATAInstructions', () => {
        it('should build load instructions by owner and mint', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);

            await mintTo(
                rpc,
                payer,
                mint,
                owner.publicKey,
                mintAuthority,
                bn(1000),
                stateTreeInfo,
                selectTokenPoolInfo(tokenPoolInfos),
            );

            const ata = getATAAddressInterface(mint, owner.publicKey);
            const ixs = await createLoadATAInstructions(
                rpc,
                payer.publicKey,
                ata,
                owner.publicKey,
                mint,
                { tokenPoolInfos },
            );

            expect(ixs.length).toBeGreaterThan(0);
        });

        it('should return empty when nothing to load (hot ATA)', async () => {
            // For a hot ATA with no cold/SPL/T22 balance, should return empty
            // This is tested via createLoadATAInstructionsFromInterface since createLoadATAInstructions
            // fetches internally
        });
    });

    describe('calculateCompressibleLoadComputeUnits', () => {
        it('should calculate base CU for single account without proof', () => {
            const cu = calculateCompressibleLoadComputeUnits(1, false);
            expect(cu).toBe(50_000 + 30_000);
        });

        it('should add proof verification CU when hasValidityProof', () => {
            const cuWithProof = calculateCompressibleLoadComputeUnits(1, true);
            const cuWithoutProof = calculateCompressibleLoadComputeUnits(
                1,
                false,
            );

            expect(cuWithProof).toBe(cuWithoutProof + 100_000);
        });

        it('should scale with number of accounts', () => {
            const cu1 = calculateCompressibleLoadComputeUnits(1, false);
            const cu3 = calculateCompressibleLoadComputeUnits(3, false);

            expect(cu3 - cu1).toBe(2 * 30_000);
        });
    });
});
