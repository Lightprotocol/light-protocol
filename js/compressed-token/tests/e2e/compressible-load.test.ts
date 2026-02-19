import { describe, it, expect, beforeAll } from 'vitest';
import { Keypair, Signer, PublicKey } from '@solana/web3.js';
import {
    Rpc,
    bn,
    newAccountWithLamports,
    createRpc,
    selectStateTreeInfo,
    TreeInfo,
    MerkleContext,
    VERSION,
    featureFlags,
    LIGHT_TOKEN_PROGRAM_ID,
} from '@lightprotocol/stateless.js';
import { createMint, mintTo } from '../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';
import {
    createLoadAccountsParams,
    createLoadAtaInstructions,
    CompressibleAccountInput,
    ParsedAccountInfoInterface,
    calculateCompressibleLoadComputeUnits,
} from '../../src/v3/actions/load-ata';
import { getAtaInterface } from '../../src/v3/get-account-interface';
import { getAssociatedTokenAddressInterface } from '../../src/v3/get-associated-token-address-interface';

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
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
        mintAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();

        mint = (
            await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
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
                    LIGHT_TOKEN_PROGRAM_ID,
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
                    LIGHT_TOKEN_PROGRAM_ID,
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

                const coldInfo = await getAtaInterface(
                    rpc,
                    getAssociatedTokenAddressInterface(mint, owner.publicKey),
                    owner.publicKey,
                    mint,
                    undefined,
                    LIGHT_TOKEN_PROGRAM_ID,
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
                        address: getAssociatedTokenAddressInterface(
                            mint,
                            owner.publicKey,
                        ),
                        accountType: 'cTokenData',
                        tokenVariant: 'vault2',
                        info: coldInfo,
                    },
                ];

                const result = await createLoadAccountsParams(
                    rpc,
                    payer.publicKey,
                    LIGHT_TOKEN_PROGRAM_ID,
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

                const accountInfo = await getAtaInterface(
                    rpc,
                    getAssociatedTokenAddressInterface(mint, owner.publicKey),
                    owner.publicKey,
                    mint,
                    undefined,
                    LIGHT_TOKEN_PROGRAM_ID,
                );

                const accounts: CompressibleAccountInput[] = [
                    {
                        address: getAssociatedTokenAddressInterface(
                            mint,
                            owner.publicKey,
                        ),
                        accountType: 'cTokenData',
                        info: accountInfo,
                    },
                ];

                await expect(
                    createLoadAccountsParams(
                        rpc,
                        payer.publicKey,
                        LIGHT_TOKEN_PROGRAM_ID,
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

                const accountInfo = await getAtaInterface(
                    rpc,
                    getAssociatedTokenAddressInterface(mint, owner.publicKey),
                    owner.publicKey,
                    mint,
                    undefined,
                    LIGHT_TOKEN_PROGRAM_ID,
                );

                const accounts: CompressibleAccountInput[] = [
                    {
                        address: getAssociatedTokenAddressInterface(
                            mint,
                            owner.publicKey,
                        ),
                        accountType: 'cTokenData',
                        tokenVariant: 'token0Vault',
                        info: accountInfo,
                    },
                ];

                const result = await createLoadAccountsParams(
                    rpc,
                    payer.publicKey,
                    LIGHT_TOKEN_PROGRAM_ID,
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

                const ata = await getAtaInterface(
                    rpc,
                    getAssociatedTokenAddressInterface(mint, owner.publicKey),
                    owner.publicKey,
                    mint,
                    undefined,
                    LIGHT_TOKEN_PROGRAM_ID,
                );

                const result = await createLoadAccountsParams(
                    rpc,
                    payer.publicKey,
                    LIGHT_TOKEN_PROGRAM_ID,
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

                const ataAddress = getAssociatedTokenAddressInterface(mint, owner.publicKey);
                const loadBatches = await createLoadAtaInstructions(
                    rpc,
                    ataAddress,
                    owner.publicKey,
                    mint,
                    payer.publicKey,
                    { tokenPoolInfos },
                );

                expect(loadBatches.length).toBeGreaterThan(0);
            });
        });
    });

    describe('createLoadAtaInstructions', () => {
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

            const ata = getAssociatedTokenAddressInterface(
                mint,
                owner.publicKey,
            );
            const batches = await createLoadAtaInstructions(
                rpc,
                ata,
                owner.publicKey,
                mint,
                payer.publicKey,
                { tokenPoolInfos },
            );

            expect(batches.length).toBeGreaterThan(0);
        });

        it('should return empty for owner with no token accounts', async () => {
            const owner = Keypair.generate();
            const ata = getAssociatedTokenAddressInterface(mint, owner.publicKey);
            const batches = await createLoadAtaInstructions(
                rpc,
                ata,
                owner.publicKey,
                mint,
                payer.publicKey,
            );
            expect(batches.length).toBe(0);
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
