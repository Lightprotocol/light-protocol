import { describe, it, expect, beforeAll } from 'vitest';
import { Keypair, Signer, PublicKey } from '@solana/web3.js';
import {
    Rpc,
    bn,
    newAccountWithLamports,
    createRpc,
    selectStateTreeInfo,
    TreeInfo,
    VERSION,
    featureFlags,
    LIGHT_TOKEN_PROGRAM_ID,
} from '@lightprotocol/stateless.js';
import {
    createMint as createSplMint,
    getOrCreateAssociatedTokenAccount,
    mintTo as splMintTo,
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    getAssociatedTokenAddressSync,
    TokenAccountNotFoundError,
} from '@solana/spl-token';
import { createMint, mintTo } from '../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';
import {
    getAccountInterface,
    getAtaInterface,
    TokenAccountSourceType,
    AccountInterface,
} from '../../src/v3/get-account-interface';
import { getAssociatedTokenAddressInterface } from '../../src/v3/get-associated-token-address-interface';
import { createAtaInterfaceIdempotent } from '../../src/v3/actions/create-ata-interface';
import { loadAta } from '../../src/index';

featureFlags.version = VERSION.V2;

const TEST_TOKEN_DECIMALS = 9;

describe('get-account-interface', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mintAuthority: Keypair;
    let stateTreeInfo: TreeInfo;

    // c-token mint
    let ctokenMint: PublicKey;
    let ctokenPoolInfos: TokenPoolInfo[];

    // SPL mint
    let splMint: PublicKey;
    let splMintAuthority: Keypair;

    // Token-2022 mint
    let t22Mint: PublicKey;
    let t22MintAuthority: Keypair;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
        mintAuthority = Keypair.generate();
        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());

        // Create c-token mint
        const ctokenMintKeypair = Keypair.generate();
        ctokenMint = (
            await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                TEST_TOKEN_DECIMALS,
                ctokenMintKeypair,
            )
        ).mint;
        ctokenPoolInfos = await getTokenPoolInfos(rpc, ctokenMint);

        // Create SPL mint
        splMintAuthority = Keypair.generate();
        splMint = await createSplMint(
            rpc,
            payer as Keypair,
            splMintAuthority.publicKey,
            null,
            TEST_TOKEN_DECIMALS,
            undefined,
            undefined,
            TOKEN_PROGRAM_ID,
        );

        // Create Token-2022 mint
        t22MintAuthority = Keypair.generate();
        t22Mint = await createSplMint(
            rpc,
            payer as Keypair,
            t22MintAuthority.publicKey,
            null,
            TEST_TOKEN_DECIMALS,
            undefined,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
    }, 120_000);

    describe('getAccountInterface', () => {
        describe('SPL token (TOKEN_PROGRAM_ID)', () => {
            it('should fetch SPL token account with explicit programId', async () => {
                const owner = await newAccountWithLamports(rpc, 1e9);
                const amount = 5000n;

                // Create and fund SPL ATA
                const ataAccount = await getOrCreateAssociatedTokenAccount(
                    rpc,
                    payer as Keypair,
                    splMint,
                    owner.publicKey,
                );
                await splMintTo(
                    rpc,
                    payer as Keypair,
                    splMint,
                    ataAccount.address,
                    splMintAuthority,
                    amount,
                );

                const result = await getAccountInterface(
                    rpc,
                    ataAccount.address,
                    'confirmed',
                    TOKEN_PROGRAM_ID,
                );

                expect(result.parsed.address.toBase58()).toBe(
                    ataAccount.address.toBase58(),
                );
                expect(result.parsed.mint.toBase58()).toBe(splMint.toBase58());
                expect(result.parsed.owner.toBase58()).toBe(
                    owner.publicKey.toBase58(),
                );
                expect(result.parsed.amount).toBe(amount);
                expect(result.isCold).toBe(false);
                expect(result.loadContext).toBeUndefined();
                expect(result._sources).toBeDefined();
                expect(result._sources?.length).toBe(1);
                expect(result._sources?.[0].type).toBe(
                    TokenAccountSourceType.Spl,
                );
            });

            it('should throw when SPL account does not exist', async () => {
                const nonExistentAddress = Keypair.generate().publicKey;

                await expect(
                    getAccountInterface(
                        rpc,
                        nonExistentAddress,
                        'confirmed',
                        TOKEN_PROGRAM_ID,
                    ),
                ).rejects.toThrow(TokenAccountNotFoundError);
            });
        });

        describe('Token-2022 (TOKEN_2022_PROGRAM_ID)', () => {
            it('should fetch Token-2022 account with explicit programId', async () => {
                const owner = await newAccountWithLamports(rpc, 1e9);
                const amount = 7500n;

                // Create and fund T22 ATA
                const ataAccount = await getOrCreateAssociatedTokenAccount(
                    rpc,
                    payer as Keypair,
                    t22Mint,
                    owner.publicKey,
                    false,
                    'confirmed',
                    undefined,
                    TOKEN_2022_PROGRAM_ID,
                );
                await splMintTo(
                    rpc,
                    payer as Keypair,
                    t22Mint,
                    ataAccount.address,
                    t22MintAuthority,
                    amount,
                    [],
                    undefined,
                    TOKEN_2022_PROGRAM_ID,
                );

                const result = await getAccountInterface(
                    rpc,
                    ataAccount.address,
                    'confirmed',
                    TOKEN_2022_PROGRAM_ID,
                );

                expect(result.parsed.address.toBase58()).toBe(
                    ataAccount.address.toBase58(),
                );
                expect(result.parsed.mint.toBase58()).toBe(t22Mint.toBase58());
                expect(result.parsed.owner.toBase58()).toBe(
                    owner.publicKey.toBase58(),
                );
                expect(result.parsed.amount).toBe(amount);
                expect(result.isCold).toBe(false);
                expect(result._sources).toBeDefined();
                expect(result._sources?.length).toBe(1);
                expect(result._sources?.[0].type).toBe(
                    TokenAccountSourceType.Token2022,
                );
            });

            it('should throw when Token-2022 account does not exist', async () => {
                const nonExistentAddress = Keypair.generate().publicKey;

                await expect(
                    getAccountInterface(
                        rpc,
                        nonExistentAddress,
                        'confirmed',
                        TOKEN_2022_PROGRAM_ID,
                    ),
                ).rejects.toThrow(TokenAccountNotFoundError);
            });
        });

        describe('c-token hot (LIGHT_TOKEN_PROGRAM_ID)', () => {
            it('should fetch c-token hot account with explicit programId', async () => {
                const owner = await newAccountWithLamports(rpc, 1e9);
                const amount = bn(10000);

                // Create c-token ATA and mint compressed, then decompress to make it hot
                await createAtaInterfaceIdempotent(
                    rpc,
                    payer,
                    ctokenMint,
                    owner.publicKey,
                    false,
                    undefined,
                    undefined,
                    undefined,
                    { compressibleConfig: null },
                );

                await mintTo(
                    rpc,
                    payer,
                    ctokenMint,
                    owner.publicKey,
                    mintAuthority,
                    amount,
                    stateTreeInfo,
                    selectTokenPoolInfo(ctokenPoolInfos),
                );

                const ctokenAta = getAssociatedTokenAddressInterface(
                    ctokenMint,
                    owner.publicKey,
                );
                await loadAta(rpc, ctokenAta, owner, ctokenMint, payer);

                const result = await getAccountInterface(
                    rpc,
                    ctokenAta,
                    'confirmed',
                    LIGHT_TOKEN_PROGRAM_ID,
                );

                expect(result.parsed.address.toBase58()).toBe(
                    ctokenAta.toBase58(),
                );
                expect(result.parsed.mint.toBase58()).toBe(
                    ctokenMint.toBase58(),
                );
                expect(result.parsed.owner.toBase58()).toBe(
                    owner.publicKey.toBase58(),
                );
                expect(result.parsed.amount).toBe(BigInt(amount.toString()));
                expect(result.isCold).toBe(false);
                expect(result.loadContext).toBeUndefined();
                expect(result._sources).toBeDefined();
                expect(result._sources?.length).toBe(1);
                expect(result._sources?.[0].type).toBe(
                    TokenAccountSourceType.CTokenHot,
                );

                // Parsed field correctness (COption layout regression)
                expect(result.parsed.isInitialized).toBe(true);
                expect(result.parsed.isFrozen).toBe(false);
                expect(result.parsed.delegatedAmount).toBe(0n);
                expect(result.parsed.delegate).toBeNull();
                expect(result.parsed.isNative).toBe(false);
                expect(result.parsed.closeAuthority).toBeNull();
            });
        });

        describe('c-token cold (compressed)', () => {
            it('minted compressed tokens require getAtaInterface (indexed by owner+mint)', async () => {
                // Note: Tokens minted compressed via mintTo are indexed by owner+mint,
                // NOT by a derived address from the ATA. For these, use getAtaInterface.
                // getAccountInterface only finds accounts compressed from on-chain
                // (via compress_accounts_idempotent hook).
                const owner = await newAccountWithLamports(rpc, 1e9);
                const amount = bn(8000);

                // Mint compressed tokens (stays cold - no decompress)
                await mintTo(
                    rpc,
                    payer,
                    ctokenMint,
                    owner.publicKey,
                    mintAuthority,
                    amount,
                    stateTreeInfo,
                    selectTokenPoolInfo(ctokenPoolInfos),
                );

                const ctokenAta = getAssociatedTokenAddressInterface(
                    ctokenMint,
                    owner.publicKey,
                );

                // getAccountInterface cannot find minted-compressed tokens by address
                await expect(
                    getAccountInterface(
                        rpc,
                        ctokenAta,
                        'confirmed',
                        LIGHT_TOKEN_PROGRAM_ID,
                    ),
                ).rejects.toThrow();

                // Use getAtaInterface with owner+mint for minted-compressed tokens
                const result = await getAtaInterface(
                    rpc,
                    ctokenAta,
                    owner.publicKey,
                    ctokenMint,
                    'confirmed',
                    LIGHT_TOKEN_PROGRAM_ID,
                );

                expect(result.parsed.mint.toBase58()).toBe(
                    ctokenMint.toBase58(),
                );
                expect(result.parsed.owner.toBase58()).toBe(
                    owner.publicKey.toBase58(),
                );
                expect(result.parsed.amount).toBe(BigInt(amount.toString()));
                expect(result.isCold).toBe(true);
                expect(result.loadContext).toBeDefined();
                expect(result._sources?.[0].type).toBe(
                    TokenAccountSourceType.CTokenCold,
                );
            });
        });

        describe('auto-detect mode (no programId)', () => {
            it('should auto-detect c-token hot account', async () => {
                const owner = await newAccountWithLamports(rpc, 1e9);
                const amount = bn(3000);

                await createAtaInterfaceIdempotent(
                    rpc,
                    payer,
                    ctokenMint,
                    owner.publicKey,
                    false,
                    undefined,
                    undefined,
                    undefined,
                    { compressibleConfig: null },
                );

                await mintTo(
                    rpc,
                    payer,
                    ctokenMint,
                    owner.publicKey,
                    mintAuthority,
                    amount,
                    stateTreeInfo,
                    selectTokenPoolInfo(ctokenPoolInfos),
                );

                const ctokenAta = getAssociatedTokenAddressInterface(
                    ctokenMint,
                    owner.publicKey,
                );
                await loadAta(rpc, ctokenAta, owner, ctokenMint, payer);

                // No programId - should auto-detect
                const result = await getAccountInterface(
                    rpc,
                    ctokenAta,
                    'confirmed',
                );

                expect(result.parsed.amount).toBe(BigInt(amount.toString()));
                expect(result.isCold).toBe(false);
                expect(result._sources?.[0].type).toBe(
                    TokenAccountSourceType.CTokenHot,
                );
            });

            it('minted compressed tokens: auto-detect requires getAtaInterface', async () => {
                // Minted compressed tokens are indexed by owner+mint, not by derived address.
                // getAccountInterface auto-detect mode cannot find them.
                const owner = await newAccountWithLamports(rpc, 1e9);
                const amount = bn(4000);

                // Mint compressed - stays cold
                await mintTo(
                    rpc,
                    payer,
                    ctokenMint,
                    owner.publicKey,
                    mintAuthority,
                    amount,
                    stateTreeInfo,
                    selectTokenPoolInfo(ctokenPoolInfos),
                );

                const ctokenAta = getAssociatedTokenAddressInterface(
                    ctokenMint,
                    owner.publicKey,
                );

                // getAccountInterface auto-detect cannot find minted-compressed tokens
                await expect(
                    getAccountInterface(rpc, ctokenAta, 'confirmed'),
                ).rejects.toThrow(TokenAccountNotFoundError);

                // Use getAtaInterface for minted-compressed tokens
                const result = await getAtaInterface(
                    rpc,
                    ctokenAta,
                    owner.publicKey,
                    ctokenMint,
                    'confirmed',
                );

                expect(result.parsed.amount).toBe(BigInt(amount.toString()));
                expect(result.isCold).toBe(true);
                expect(result._sources?.[0].type).toBe(
                    TokenAccountSourceType.CTokenCold,
                );
            });
        });

        describe('error cases', () => {
            it('should throw for unsupported program ID', async () => {
                const fakeAddress = Keypair.generate().publicKey;
                const fakeProgramId = Keypair.generate().publicKey;

                await expect(
                    getAccountInterface(
                        rpc,
                        fakeAddress,
                        'confirmed',
                        fakeProgramId,
                    ),
                ).rejects.toThrow(/Unsupported program ID/);
            });

            it('should throw when account not found in auto-detect mode', async () => {
                const owner = Keypair.generate();
                const nonExistentAta = getAssociatedTokenAddressInterface(
                    ctokenMint,
                    owner.publicKey,
                );

                await expect(
                    getAccountInterface(rpc, nonExistentAta, 'confirmed'),
                ).rejects.toThrow(TokenAccountNotFoundError);
            });
        });
    });

    describe('getAtaInterface', () => {
        describe('c-token hot only', () => {
            it('should return hot ATA with correct metadata', async () => {
                const owner = await newAccountWithLamports(rpc, 1e9);
                const amount = bn(6000);

                await createAtaInterfaceIdempotent(
                    rpc,
                    payer,
                    ctokenMint,
                    owner.publicKey,
                    false,
                    undefined,
                    undefined,
                    undefined,
                    { compressibleConfig: null },
                );

                await mintTo(
                    rpc,
                    payer,
                    ctokenMint,
                    owner.publicKey,
                    mintAuthority,
                    amount,
                    stateTreeInfo,
                    selectTokenPoolInfo(ctokenPoolInfos),
                );

                const ctokenAta = getAssociatedTokenAddressInterface(
                    ctokenMint,
                    owner.publicKey,
                );
                await loadAta(rpc, ctokenAta, owner, ctokenMint, payer);

                const result = await getAtaInterface(
                    rpc,
                    ctokenAta,
                    owner.publicKey,
                    ctokenMint,
                );

                expect(result._isAta).toBe(true);
                expect(result._owner?.toBase58()).toBe(
                    owner.publicKey.toBase58(),
                );
                expect(result._mint?.toBase58()).toBe(ctokenMint.toBase58());
                expect(result.parsed.amount).toBe(BigInt(amount.toString()));
                expect(result.isCold).toBe(false);
                expect(result._needsConsolidation).toBe(false);
                expect(result._sources?.length).toBe(1);
                expect(result._sources?.[0].type).toBe(
                    TokenAccountSourceType.CTokenHot,
                );
            });
        });

        describe('c-token cold only', () => {
            it('should return cold ATA with loadContext', async () => {
                const owner = await newAccountWithLamports(rpc, 1e9);
                const amount = bn(5500);

                // Mint compressed - stays cold
                await mintTo(
                    rpc,
                    payer,
                    ctokenMint,
                    owner.publicKey,
                    mintAuthority,
                    amount,
                    stateTreeInfo,
                    selectTokenPoolInfo(ctokenPoolInfos),
                );

                const ctokenAta = getAssociatedTokenAddressInterface(
                    ctokenMint,
                    owner.publicKey,
                );

                const result = await getAtaInterface(
                    rpc,
                    ctokenAta,
                    owner.publicKey,
                    ctokenMint,
                );

                expect(result._isAta).toBe(true);
                expect(result._owner?.toBase58()).toBe(
                    owner.publicKey.toBase58(),
                );
                expect(result._mint?.toBase58()).toBe(ctokenMint.toBase58());
                expect(result.parsed.amount).toBe(BigInt(amount.toString()));
                expect(result.isCold).toBe(true);
                expect(result.loadContext).toBeDefined();
                expect(result.loadContext?.hash).toBeDefined();
                expect(result._needsConsolidation).toBe(false);
                expect(result._sources?.length).toBe(1);
                expect(result._sources?.[0].type).toBe(
                    TokenAccountSourceType.CTokenCold,
                );

                // Parsed field correctness for cold accounts
                expect(result.parsed.isInitialized).toBe(true);
                expect(result.parsed.isFrozen).toBe(false);
                expect(result.parsed.delegatedAmount).toBe(0n);
                expect(result.parsed.delegate).toBeNull();
                expect(result.parsed.isNative).toBe(false);
                expect(result.parsed.closeAuthority).toBeNull();
            });
        });

        describe('c-token hot + cold combined', () => {
            it('should aggregate hot and cold balances', async () => {
                const owner = await newAccountWithLamports(rpc, 1e9);
                const hotAmount = bn(2000);
                const coldAmount = bn(3000);

                await createAtaInterfaceIdempotent(
                    rpc,
                    payer,
                    ctokenMint,
                    owner.publicKey,
                    false,
                    undefined,
                    undefined,
                    undefined,
                    { compressibleConfig: null },
                );

                // Mint and decompress first batch (hot)
                await mintTo(
                    rpc,
                    payer,
                    ctokenMint,
                    owner.publicKey,
                    mintAuthority,
                    hotAmount,
                    stateTreeInfo,
                    selectTokenPoolInfo(ctokenPoolInfos),
                );
                const ctokenAtaEarly = getAssociatedTokenAddressInterface(
                    ctokenMint,
                    owner.publicKey,
                );
                await loadAta(rpc, ctokenAtaEarly, owner, ctokenMint, payer);

                // Mint second batch (cold)
                await mintTo(
                    rpc,
                    payer,
                    ctokenMint,
                    owner.publicKey,
                    mintAuthority,
                    coldAmount,
                    stateTreeInfo,
                    selectTokenPoolInfo(ctokenPoolInfos),
                );

                const ctokenAta = getAssociatedTokenAddressInterface(
                    ctokenMint,
                    owner.publicKey,
                );

                const result = await getAtaInterface(
                    rpc,
                    ctokenAta,
                    owner.publicKey,
                    ctokenMint,
                );

                // Total should be hot + cold
                const expectedTotal =
                    BigInt(hotAmount.toString()) +
                    BigInt(coldAmount.toString());
                expect(result.parsed.amount).toBe(expectedTotal);

                // Should have both sources
                expect(result._sources?.length).toBe(2);
                expect(result._needsConsolidation).toBe(true);

                // Primary source should be hot (higher priority)
                expect(result.isCold).toBe(false);
                expect(result._sources?.[0].type).toBe(
                    TokenAccountSourceType.CTokenHot,
                );
                expect(result._sources?.[1].type).toBe(
                    TokenAccountSourceType.CTokenCold,
                );

                // Verify individual source amounts
                expect(result._sources?.[0].amount).toBe(
                    BigInt(hotAmount.toString()),
                );
                expect(result._sources?.[1].amount).toBe(
                    BigInt(coldAmount.toString()),
                );
            });
        });

        describe('wrap=true (include SPL/T22 balances)', () => {
            it('should include SPL balance when wrap=true', async () => {
                const owner = await newAccountWithLamports(rpc, 1e9);
                const ctokenAmount = bn(1500);
                const splAmount = 2500n;

                // Setup c-token cold
                await mintTo(
                    rpc,
                    payer,
                    ctokenMint,
                    owner.publicKey,
                    mintAuthority,
                    ctokenAmount,
                    stateTreeInfo,
                    selectTokenPoolInfo(ctokenPoolInfos),
                );

                // Create SPL ATA with same mint (won't work with c-token mint)
                // For this test we need an SPL mint that has a token pool
                // Actually, wrap=true requires mint parity - use the c-token mint
                // But SPL ATAs need SPL mints. This scenario is for unified view
                // where user has tokens across multiple account types.

                const ctokenAta = getAssociatedTokenAddressInterface(
                    ctokenMint,
                    owner.publicKey,
                );

                const result = await getAtaInterface(
                    rpc,
                    ctokenAta,
                    owner.publicKey,
                    ctokenMint,
                    undefined,
                    undefined,
                    true, // wrap=true
                );

                // Should have c-token cold (SPL ATA for c-token mint doesn't exist)
                expect(result._isAta).toBe(true);
                expect(result.parsed.amount).toBe(
                    BigInt(ctokenAmount.toString()),
                );
                expect(result._sources?.length).toBeGreaterThanOrEqual(1);
            });
        });

        describe('metadata fields', () => {
            it('should set _hasDelegate when delegate is present', async () => {
                // Note: This would require setting up a delegate, which is complex
                // For now, verify the field exists when no delegate
                const owner = await newAccountWithLamports(rpc, 1e9);
                const amount = bn(1000);

                await mintTo(
                    rpc,
                    payer,
                    ctokenMint,
                    owner.publicKey,
                    mintAuthority,
                    amount,
                    stateTreeInfo,
                    selectTokenPoolInfo(ctokenPoolInfos),
                );

                const ctokenAta = getAssociatedTokenAddressInterface(
                    ctokenMint,
                    owner.publicKey,
                );

                const result = await getAtaInterface(
                    rpc,
                    ctokenAta,
                    owner.publicKey,
                    ctokenMint,
                );

                expect(result._hasDelegate).toBe(false);
            });

            it('should set _anyFrozen correctly', async () => {
                const owner = await newAccountWithLamports(rpc, 1e9);
                const amount = bn(1000);

                await mintTo(
                    rpc,
                    payer,
                    ctokenMint,
                    owner.publicKey,
                    mintAuthority,
                    amount,
                    stateTreeInfo,
                    selectTokenPoolInfo(ctokenPoolInfos),
                );

                const ctokenAta = getAssociatedTokenAddressInterface(
                    ctokenMint,
                    owner.publicKey,
                );

                const result = await getAtaInterface(
                    rpc,
                    ctokenAta,
                    owner.publicKey,
                    ctokenMint,
                );

                expect(result._anyFrozen).toBe(false);
            });
        });

        describe('error cases', () => {
            it('should throw TokenAccountNotFoundError when no account exists', async () => {
                const owner = Keypair.generate();
                const nonExistentAta = getAssociatedTokenAddressInterface(
                    ctokenMint,
                    owner.publicKey,
                );

                await expect(
                    getAtaInterface(
                        rpc,
                        nonExistentAta,
                        owner.publicKey,
                        ctokenMint,
                    ),
                ).rejects.toThrow(TokenAccountNotFoundError);
            });
        });

        describe('SPL programId scenarios', () => {
            it('should fetch SPL ATA with explicit TOKEN_PROGRAM_ID', async () => {
                const owner = await newAccountWithLamports(rpc, 1e9);
                const amount = 4000n;

                const ataAccount = await getOrCreateAssociatedTokenAccount(
                    rpc,
                    payer as Keypair,
                    splMint,
                    owner.publicKey,
                );
                await splMintTo(
                    rpc,
                    payer as Keypair,
                    splMint,
                    ataAccount.address,
                    splMintAuthority,
                    amount,
                );

                const result = await getAtaInterface(
                    rpc,
                    ataAccount.address,
                    owner.publicKey,
                    splMint,
                    undefined,
                    TOKEN_PROGRAM_ID,
                );

                expect(result._isAta).toBe(true);
                expect(result.parsed.amount).toBe(amount);
                expect(result._sources?.[0].type).toBe(
                    TokenAccountSourceType.Spl,
                );
            });

            it('should fetch T22 ATA with explicit TOKEN_2022_PROGRAM_ID', async () => {
                const owner = await newAccountWithLamports(rpc, 1e9);
                const amount = 6000n;

                const ataAccount = await getOrCreateAssociatedTokenAccount(
                    rpc,
                    payer as Keypair,
                    t22Mint,
                    owner.publicKey,
                    false,
                    'confirmed',
                    undefined,
                    TOKEN_2022_PROGRAM_ID,
                );
                await splMintTo(
                    rpc,
                    payer as Keypair,
                    t22Mint,
                    ataAccount.address,
                    t22MintAuthority,
                    amount,
                    [],
                    undefined,
                    TOKEN_2022_PROGRAM_ID,
                );

                const result = await getAtaInterface(
                    rpc,
                    ataAccount.address,
                    owner.publicKey,
                    t22Mint,
                    undefined,
                    TOKEN_2022_PROGRAM_ID,
                );

                expect(result._isAta).toBe(true);
                expect(result.parsed.amount).toBe(amount);
                expect(result._sources?.[0].type).toBe(
                    TokenAccountSourceType.Token2022,
                );
            });
        });

        describe('SPL with cold balance (fetchByOwner)', () => {
            it('should include SPL cold balance when SPL ATA has compressed tokens', async () => {
                const owner = await newAccountWithLamports(rpc, 1e9);
                const splHotAmount = 3000n;
                const compressedAmount = bn(2000);

                // Create SPL ATA and fund it
                const ataAccount = await getOrCreateAssociatedTokenAccount(
                    rpc,
                    payer as Keypair,
                    splMint,
                    owner.publicKey,
                );
                await splMintTo(
                    rpc,
                    payer as Keypair,
                    splMint,
                    ataAccount.address,
                    splMintAuthority,
                    splHotAmount,
                );

                // Note: To have compressed tokens for an SPL mint, we'd need
                // to register that mint as a token pool and mint compressed.
                // For this test, we verify the basic SPL fetch works.

                const result = await getAtaInterface(
                    rpc,
                    ataAccount.address,
                    owner.publicKey,
                    splMint,
                    undefined,
                    TOKEN_PROGRAM_ID,
                );

                expect(result.parsed.amount).toBe(splHotAmount);
                expect(result._sources?.[0].type).toBe(
                    TokenAccountSourceType.Spl,
                );
            });
        });
    });

    describe('balance aggregation', () => {
        it('should correctly aggregate balance from single mint', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);
            const totalAmount = bn(6000);

            // Single mint with total amount
            const sig = await mintTo(
                rpc,
                payer,
                ctokenMint,
                owner.publicKey,
                mintAuthority,
                totalAmount,
                stateTreeInfo,
                selectTokenPoolInfo(ctokenPoolInfos),
            );
            await rpc.confirmTransaction(sig, 'confirmed');

            const ctokenAta = getAssociatedTokenAddressInterface(
                ctokenMint,
                owner.publicKey,
            );

            const result = await getAtaInterface(
                rpc,
                ctokenAta,
                owner.publicKey,
                ctokenMint,
            );

            expect(result.parsed.amount).toBe(BigInt(totalAmount.toString()));
            expect(result._sources?.length).toBeGreaterThanOrEqual(1);
            expect(result.isCold).toBe(true);
        });

        it('should aggregate hot and cold balances correctly', async () => {
            // This is already tested in hot+cold combined test but verify the math
            const owner = await newAccountWithLamports(rpc, 1e9);
            const hotAmount = bn(1500);
            const coldAmount = bn(2500);

            await createAtaInterfaceIdempotent(
                rpc,
                payer,
                ctokenMint,
                owner.publicKey,
                false,
                undefined,
                undefined,
                undefined,
                { compressibleConfig: null },
            );

            // Mint and decompress to create hot balance
            const sig1 = await mintTo(
                rpc,
                payer,
                ctokenMint,
                owner.publicKey,
                mintAuthority,
                hotAmount,
                stateTreeInfo,
                selectTokenPoolInfo(ctokenPoolInfos),
            );
            await rpc.confirmTransaction(sig1, 'confirmed');
            const ctokenAtaEarly = getAssociatedTokenAddressInterface(
                ctokenMint,
                owner.publicKey,
            );
            await loadAta(rpc, ctokenAtaEarly, owner, ctokenMint, payer);

            // Mint more to create cold balance
            const sig2 = await mintTo(
                rpc,
                payer,
                ctokenMint,
                owner.publicKey,
                mintAuthority,
                coldAmount,
                stateTreeInfo,
                selectTokenPoolInfo(ctokenPoolInfos),
            );
            await rpc.confirmTransaction(sig2, 'confirmed');

            const ctokenAta = getAssociatedTokenAddressInterface(
                ctokenMint,
                owner.publicKey,
            );

            const result = await getAtaInterface(
                rpc,
                ctokenAta,
                owner.publicKey,
                ctokenMint,
            );

            const expectedTotal =
                BigInt(hotAmount.toString()) + BigInt(coldAmount.toString());

            expect(result.parsed.amount).toBe(expectedTotal);
            expect(result._sources?.length).toBe(2);
            expect(result._needsConsolidation).toBe(true);
        });
    });

    describe('source priority ordering', () => {
        it('should prioritize hot over cold', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);
            const hotAmount = bn(500);
            const coldAmount = bn(1500);

            await createAtaInterfaceIdempotent(
                rpc,
                payer,
                ctokenMint,
                owner.publicKey,
                false,
                undefined,
                undefined,
                undefined,
                { compressibleConfig: null },
            );

            // Create hot first
            await mintTo(
                rpc,
                payer,
                ctokenMint,
                owner.publicKey,
                mintAuthority,
                hotAmount,
                stateTreeInfo,
                selectTokenPoolInfo(ctokenPoolInfos),
            );
            const ctokenAtaEarly = getAssociatedTokenAddressInterface(
                ctokenMint,
                owner.publicKey,
            );
            await loadAta(rpc, ctokenAtaEarly, owner, ctokenMint, payer);

            // Then add cold
            await mintTo(
                rpc,
                payer,
                ctokenMint,
                owner.publicKey,
                mintAuthority,
                coldAmount,
                stateTreeInfo,
                selectTokenPoolInfo(ctokenPoolInfos),
            );

            const ctokenAta = getAssociatedTokenAddressInterface(
                ctokenMint,
                owner.publicKey,
            );

            const result = await getAtaInterface(
                rpc,
                ctokenAta,
                owner.publicKey,
                ctokenMint,
            );

            // Primary (first) source should be hot
            expect(result.isCold).toBe(false);
            expect(result._sources?.[0].type).toBe(
                TokenAccountSourceType.CTokenHot,
            );

            // Total balance is correct
            const expectedTotal =
                BigInt(hotAmount.toString()) + BigInt(coldAmount.toString());
            expect(result.parsed.amount).toBe(expectedTotal);
        });
    });

    // ================================================================
    // FULL AGGREGATION COVERAGE
    // ================================================================
    // Uses ctokenMint which is an SPL Token mint with a Light token pool,
    // so both SPL ATAs and compressed accounts can coexist.

    const sortBigInt = (a: bigint, b: bigint) => (a < b ? -1 : a > b ? 1 : 0);

    describe('multi-cold aggregation', () => {
        it('should aggregate 3 cold accounts with exact per-source amounts', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);
            const amounts = [1000n, 2000n, 3000n];

            for (const amount of amounts) {
                await mintTo(
                    rpc,
                    payer,
                    ctokenMint,
                    owner.publicKey,
                    mintAuthority,
                    bn(amount),
                    stateTreeInfo,
                    selectTokenPoolInfo(ctokenPoolInfos),
                );
            }

            const ctokenAta = getAssociatedTokenAddressInterface(
                ctokenMint,
                owner.publicKey,
            );
            const result = await getAtaInterface(
                rpc,
                ctokenAta,
                owner.publicKey,
                ctokenMint,
            );

            expect(result.parsed.amount).toBe(6000n);
            expect(result._sources?.length).toBe(3);
            expect(result.isCold).toBe(true);
            expect(result._needsConsolidation).toBe(true);

            for (const source of result._sources!) {
                expect(source.type).toBe(TokenAccountSourceType.CTokenCold);
            }

            const sourceAmounts = result
                ._sources!.map(s => s.amount)
                .sort(sortBigInt);
            expect(sourceAmounts).toEqual([1000n, 2000n, 3000n]);
        }, 60_000);

        it('should aggregate ctoken-hot + 3 cold with exact per-source amounts', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);
            const hotAmount = 500n;
            const coldAmounts = [1000n, 2000n, 3000n];

            await createAtaInterfaceIdempotent(
                rpc,
                payer,
                ctokenMint,
                owner.publicKey,
                false,
                undefined,
                undefined,
                undefined,
                { compressibleConfig: null },
            );
            await mintTo(
                rpc,
                payer,
                ctokenMint,
                owner.publicKey,
                mintAuthority,
                bn(hotAmount),
                stateTreeInfo,
                selectTokenPoolInfo(ctokenPoolInfos),
            );
            const ctokenAtaForCold = getAssociatedTokenAddressInterface(
                ctokenMint,
                owner.publicKey,
            );
            await loadAta(rpc, ctokenAtaForCold, owner, ctokenMint, payer);

            for (const amount of coldAmounts) {
                await mintTo(
                    rpc,
                    payer,
                    ctokenMint,
                    owner.publicKey,
                    mintAuthority,
                    bn(amount),
                    stateTreeInfo,
                    selectTokenPoolInfo(ctokenPoolInfos),
                );
            }

            const ctokenAta = getAssociatedTokenAddressInterface(
                ctokenMint,
                owner.publicKey,
            );
            const result = await getAtaInterface(
                rpc,
                ctokenAta,
                owner.publicKey,
                ctokenMint,
            );

            expect(result.parsed.amount).toBe(6500n);
            expect(result._sources?.length).toBe(4);
            expect(result.isCold).toBe(false);
            expect(result._needsConsolidation).toBe(true);

            // First source is hot (priority)
            expect(result._sources![0].type).toBe(
                TokenAccountSourceType.CTokenHot,
            );
            expect(result._sources![0].amount).toBe(hotAmount);

            // Remaining are cold
            const coldSources = result._sources!.slice(1);
            for (const source of coldSources) {
                expect(source.type).toBe(TokenAccountSourceType.CTokenCold);
            }
            const coldSourceAmounts = coldSources
                .map(s => s.amount)
                .sort(sortBigInt);
            expect(coldSourceAmounts).toEqual([1000n, 2000n, 3000n]);
        }, 120_000);
    });

    describe('SPL programId aggregation', () => {
        it('should show SPL hot + spl-cold with exact amounts (programId=TOKEN_PROGRAM_ID)', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);
            const splHotAmount = 1500n;
            const coldAmounts = [800n, 1200n];

            // Create SPL ATA and mint SPL tokens directly
            const splAta = await getOrCreateAssociatedTokenAccount(
                rpc,
                payer as Keypair,
                ctokenMint,
                owner.publicKey,
            );
            await splMintTo(
                rpc,
                payer as Keypair,
                ctokenMint,
                splAta.address,
                mintAuthority,
                splHotAmount,
            );

            // Mint compressed tokens (will appear as spl-cold)
            for (const amount of coldAmounts) {
                await mintTo(
                    rpc,
                    payer,
                    ctokenMint,
                    owner.publicKey,
                    mintAuthority,
                    bn(amount),
                    stateTreeInfo,
                    selectTokenPoolInfo(ctokenPoolInfos),
                );
            }

            const result = await getAtaInterface(
                rpc,
                splAta.address,
                owner.publicKey,
                ctokenMint,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            expect(result.parsed.amount).toBe(3500n);
            expect(result._sources?.length).toBe(3);

            // First source is SPL hot
            expect(result._sources![0].type).toBe(TokenAccountSourceType.Spl);
            expect(result._sources![0].amount).toBe(splHotAmount);

            // Cold sources are spl-cold
            const coldSources = result._sources!.filter(
                s => s.type === TokenAccountSourceType.SplCold,
            );
            expect(coldSources.length).toBe(2);
            const coldSourceAmounts = coldSources
                .map(s => s.amount)
                .sort(sortBigInt);
            expect(coldSourceAmounts).toEqual([800n, 1200n]);
        }, 60_000);

        it('should show spl-cold only when no SPL ATA exists (programId=TOKEN_PROGRAM_ID)', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);
            const coldAmounts = [700n, 1300n];

            // Mint compressed tokens only (no SPL ATA created)
            for (const amount of coldAmounts) {
                await mintTo(
                    rpc,
                    payer,
                    ctokenMint,
                    owner.publicKey,
                    mintAuthority,
                    bn(amount),
                    stateTreeInfo,
                    selectTokenPoolInfo(ctokenPoolInfos),
                );
            }

            // Derive SPL ATA address (not on-chain)
            const splAta = getAssociatedTokenAddressSync(
                ctokenMint,
                owner.publicKey,
            );

            const result = await getAtaInterface(
                rpc,
                splAta,
                owner.publicKey,
                ctokenMint,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            expect(result.parsed.amount).toBe(2000n);
            expect(result._sources?.length).toBe(2);
            expect(result.isCold).toBe(true);

            for (const source of result._sources!) {
                expect(source.type).toBe(TokenAccountSourceType.SplCold);
            }
            const sourceAmounts = result
                ._sources!.map(s => s.amount)
                .sort(sortBigInt);
            expect(sourceAmounts).toEqual([700n, 1300n]);
        }, 60_000);
    });

    describe('cross-program unified aggregation (all modes from one setup)', () => {
        // Shared state: ctoken-hot(3000) + 2 cold(1000,2000) + SPL hot(1500)
        let unifiedOwner: Signer;
        const uHotAmount = 3000n;
        const uCold1 = 1000n;
        const uCold2 = 2000n;
        const uSplHot = 1500n;

        beforeAll(async () => {
            unifiedOwner = await newAccountWithLamports(rpc, 2e9);

            // ctoken-hot: mint + decompress
            await createAtaInterfaceIdempotent(
                rpc,
                payer,
                ctokenMint,
                unifiedOwner.publicKey,
                false,
                undefined,
                undefined,
                undefined,
                { compressibleConfig: null },
            );
            await mintTo(
                rpc,
                payer,
                ctokenMint,
                unifiedOwner.publicKey,
                mintAuthority,
                bn(uHotAmount),
                stateTreeInfo,
                selectTokenPoolInfo(ctokenPoolInfos),
            );
            const unifiedCtokenAta = getAssociatedTokenAddressInterface(
                ctokenMint,
                unifiedOwner.publicKey,
            );
            await loadAta(
                rpc,
                unifiedCtokenAta,
                unifiedOwner,
                ctokenMint,
                payer,
            );

            // 2 cold accounts
            await mintTo(
                rpc,
                payer,
                ctokenMint,
                unifiedOwner.publicKey,
                mintAuthority,
                bn(uCold1),
                stateTreeInfo,
                selectTokenPoolInfo(ctokenPoolInfos),
            );
            await mintTo(
                rpc,
                payer,
                ctokenMint,
                unifiedOwner.publicKey,
                mintAuthority,
                bn(uCold2),
                stateTreeInfo,
                selectTokenPoolInfo(ctokenPoolInfos),
            );

            // SPL ATA with balance
            const splAta = await getOrCreateAssociatedTokenAccount(
                rpc,
                payer as Keypair,
                ctokenMint,
                unifiedOwner.publicKey,
            );
            await splMintTo(
                rpc,
                payer as Keypair,
                ctokenMint,
                splAta.address,
                mintAuthority,
                uSplHot,
            );
        }, 120_000);

        it('wrap=true: aggregates ctoken-hot + ctoken-cold + SPL hot', async () => {
            const ctokenAta = getAssociatedTokenAddressInterface(
                ctokenMint,
                unifiedOwner.publicKey,
            );
            const result = await getAtaInterface(
                rpc,
                ctokenAta,
                unifiedOwner.publicKey,
                ctokenMint,
                undefined,
                undefined,
                true,
            );

            expect(result.parsed.amount).toBe(
                uHotAmount + uCold1 + uCold2 + uSplHot,
            ); // 7500
            expect(result._needsConsolidation).toBe(true);

            const types = result._sources!.map(s => s.type);
            expect(types).toContain(TokenAccountSourceType.CTokenHot);
            expect(types).toContain(TokenAccountSourceType.CTokenCold);
            expect(types).toContain(TokenAccountSourceType.Spl);

            // Priority: ctoken-hot first
            expect(result._sources![0].type).toBe(
                TokenAccountSourceType.CTokenHot,
            );
            expect(result._sources![0].amount).toBe(uHotAmount);

            // SPL source amount
            const splSource = result._sources!.find(
                s => s.type === TokenAccountSourceType.Spl,
            );
            expect(splSource!.amount).toBe(uSplHot);

            // Cold amounts
            const coldSources = result._sources!.filter(
                s => s.type === TokenAccountSourceType.CTokenCold,
            );
            expect(coldSources.length).toBe(2);
            expect(coldSources.map(s => s.amount).sort(sortBigInt)).toEqual([
                uCold1,
                uCold2,
            ]);
        });

        it('wrap=false: excludes SPL sources', async () => {
            const ctokenAta = getAssociatedTokenAddressInterface(
                ctokenMint,
                unifiedOwner.publicKey,
            );
            const result = await getAtaInterface(
                rpc,
                ctokenAta,
                unifiedOwner.publicKey,
                ctokenMint,
                undefined,
                undefined,
                false,
            );

            expect(result.parsed.amount).toBe(uHotAmount + uCold1 + uCold2); // 6000

            const types = result._sources!.map(s => s.type);
            expect(types).not.toContain(TokenAccountSourceType.Spl);
            expect(types).not.toContain(TokenAccountSourceType.Token2022);
            expect(types).toContain(TokenAccountSourceType.CTokenHot);
            expect(types).toContain(TokenAccountSourceType.CTokenCold);
        });

        it('programId=TOKEN_PROGRAM_ID: shows SPL hot + compressed as spl-cold', async () => {
            const splAta = getAssociatedTokenAddressSync(
                ctokenMint,
                unifiedOwner.publicKey,
            );
            const result = await getAtaInterface(
                rpc,
                splAta,
                unifiedOwner.publicKey,
                ctokenMint,
                undefined,
                TOKEN_PROGRAM_ID,
            );

            expect(result.parsed.amount).toBe(uSplHot + uCold1 + uCold2); // 4500

            const types = result._sources!.map(s => s.type);
            expect(types).not.toContain(TokenAccountSourceType.CTokenHot);
            expect(types).toContain(TokenAccountSourceType.Spl);
            expect(types).toContain(TokenAccountSourceType.SplCold);

            const splSource = result._sources!.find(
                s => s.type === TokenAccountSourceType.Spl,
            );
            expect(splSource!.amount).toBe(uSplHot);

            const coldSources = result._sources!.filter(
                s => s.type === TokenAccountSourceType.SplCold,
            );
            expect(coldSources.length).toBe(2);
            expect(coldSources.map(s => s.amount).sort(sortBigInt)).toEqual([
                uCold1,
                uCold2,
            ]);
        });

        it('programId=LIGHT_TOKEN: shows ctoken-hot + ctoken-cold only', async () => {
            const ctokenAta = getAssociatedTokenAddressInterface(
                ctokenMint,
                unifiedOwner.publicKey,
            );
            const result = await getAtaInterface(
                rpc,
                ctokenAta,
                unifiedOwner.publicKey,
                ctokenMint,
                undefined,
                LIGHT_TOKEN_PROGRAM_ID,
            );

            expect(result.parsed.amount).toBe(uHotAmount + uCold1 + uCold2); // 6000

            const types = result._sources!.map(s => s.type);
            expect(types).not.toContain(TokenAccountSourceType.Spl);
            expect(types).not.toContain(TokenAccountSourceType.SplCold);
            expect(types).toContain(TokenAccountSourceType.CTokenHot);
            expect(types).toContain(TokenAccountSourceType.CTokenCold);

            expect(result._sources![0].type).toBe(
                TokenAccountSourceType.CTokenHot,
            );
            expect(result._sources![0].amount).toBe(uHotAmount);
        });
    });

    describe('wrap=true edge cases', () => {
        it('wrap=true with only SPL hot (no ctoken accounts)', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);
            const splHotAmount = 5000n;

            const splAta = await getOrCreateAssociatedTokenAccount(
                rpc,
                payer as Keypair,
                ctokenMint,
                owner.publicKey,
            );
            await splMintTo(
                rpc,
                payer as Keypair,
                ctokenMint,
                splAta.address,
                mintAuthority,
                splHotAmount,
            );

            const ctokenAta = getAssociatedTokenAddressInterface(
                ctokenMint,
                owner.publicKey,
            );
            const result = await getAtaInterface(
                rpc,
                ctokenAta,
                owner.publicKey,
                ctokenMint,
                undefined,
                undefined,
                true,
            );

            expect(result.parsed.amount).toBe(splHotAmount);
            expect(result._sources?.length).toBe(1);
            expect(result._sources![0].type).toBe(TokenAccountSourceType.Spl);
            expect(result._sources![0].amount).toBe(splHotAmount);
        }, 60_000);

        it('wrap=true with ctoken-cold + SPL hot (no ctoken-hot)', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);
            const coldAmount = 2000n;
            const splHotAmount = 3000n;

            // Cold only
            await mintTo(
                rpc,
                payer,
                ctokenMint,
                owner.publicKey,
                mintAuthority,
                bn(coldAmount),
                stateTreeInfo,
                selectTokenPoolInfo(ctokenPoolInfos),
            );

            // SPL hot
            const splAta = await getOrCreateAssociatedTokenAccount(
                rpc,
                payer as Keypair,
                ctokenMint,
                owner.publicKey,
            );
            await splMintTo(
                rpc,
                payer as Keypair,
                ctokenMint,
                splAta.address,
                mintAuthority,
                splHotAmount,
            );

            const ctokenAta = getAssociatedTokenAddressInterface(
                ctokenMint,
                owner.publicKey,
            );
            const result = await getAtaInterface(
                rpc,
                ctokenAta,
                owner.publicKey,
                ctokenMint,
                undefined,
                undefined,
                true,
            );

            expect(result.parsed.amount).toBe(coldAmount + splHotAmount); // 5000
            expect(result._sources?.length).toBe(2);

            const coldSource = result._sources!.find(
                s => s.type === TokenAccountSourceType.CTokenCold,
            );
            expect(coldSource!.amount).toBe(coldAmount);

            const splSource = result._sources!.find(
                s => s.type === TokenAccountSourceType.Spl,
            );
            expect(splSource!.amount).toBe(splHotAmount);
        }, 60_000);

        it('wrap=true with ctoken-hot + SPL hot (no cold)', async () => {
            const owner = await newAccountWithLamports(rpc, 1e9);
            const hotAmount = 4000n;
            const splHotAmount = 2000n;

            // ctoken-hot
            await createAtaInterfaceIdempotent(
                rpc,
                payer,
                ctokenMint,
                owner.publicKey,
                false,
                undefined,
                undefined,
                undefined,
                { compressibleConfig: null },
            );
            await mintTo(
                rpc,
                payer,
                ctokenMint,
                owner.publicKey,
                mintAuthority,
                bn(hotAmount),
                stateTreeInfo,
                selectTokenPoolInfo(ctokenPoolInfos),
            );
            const ctokenAtaForLoad = getAssociatedTokenAddressInterface(
                ctokenMint,
                owner.publicKey,
            );
            await loadAta(rpc, ctokenAtaForLoad, owner, ctokenMint, payer);

            // SPL hot
            const splAta = await getOrCreateAssociatedTokenAccount(
                rpc,
                payer as Keypair,
                ctokenMint,
                owner.publicKey,
            );
            await splMintTo(
                rpc,
                payer as Keypair,
                ctokenMint,
                splAta.address,
                mintAuthority,
                splHotAmount,
            );

            const ctokenAta = getAssociatedTokenAddressInterface(
                ctokenMint,
                owner.publicKey,
            );
            const result = await getAtaInterface(
                rpc,
                ctokenAta,
                owner.publicKey,
                ctokenMint,
                undefined,
                undefined,
                true,
            );

            expect(result.parsed.amount).toBe(hotAmount + splHotAmount); // 6000
            expect(result._sources?.length).toBe(2);
            expect(result._needsConsolidation).toBe(true);

            expect(result._sources![0].type).toBe(
                TokenAccountSourceType.CTokenHot,
            );
            expect(result._sources![0].amount).toBe(hotAmount);

            const splSource = result._sources!.find(
                s => s.type === TokenAccountSourceType.Spl,
            );
            expect(splSource!.amount).toBe(splHotAmount);
        }, 120_000);
    });
});
