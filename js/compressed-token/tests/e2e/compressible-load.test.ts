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
} from '@lightprotocol/stateless.js';
import { createMint, mintTo } from '../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';
import { createLoadAtaInstructions } from '../../src/v3/actions/load-ata';
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
                { splInterfaceInfos: tokenPoolInfos },
            );

            expect(batches.length).toBeGreaterThan(0);
        });

        it('should return empty for owner with no token accounts', async () => {
            const owner = Keypair.generate();
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
            );
            expect(batches.length).toBe(0);
        });
    });

});
