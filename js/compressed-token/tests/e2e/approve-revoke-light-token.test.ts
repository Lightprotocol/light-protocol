/**
 * E2E tests for createLightTokenApproveInstruction and
 * createLightTokenRevokeInstruction (native discriminator 4/5).
 *
 * These instructions operate on decompressed light-token (hot) accounts,
 * mimicking SPL Token approve/revoke semantics through the LightToken program.
 */
import { describe, it, expect, beforeAll } from 'vitest';
import { Keypair, PublicKey, Signer, SystemProgram } from '@solana/web3.js';
import {
    Rpc,
    bn,
    newAccountWithLamports,
    createRpc,
    selectStateTreeInfo,
    TreeInfo,
    VERSION,
    featureFlags,
    buildAndSignTx,
    sendAndConfirmTx,
    LIGHT_TOKEN_PROGRAM_ID,
} from '@lightprotocol/stateless.js';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { createMint, mintTo } from '../../src/actions';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';
import { getAssociatedTokenAddressInterface } from '../../src/v3/get-associated-token-address-interface';
import { createAtaInterfaceIdempotent } from '../../src/v3/actions/create-ata-interface';
import { loadAta } from '../../src/v3/actions/load-ata';
import {
    createLightTokenApproveInstruction,
    createLightTokenRevokeInstruction,
} from '../../src/v3/instructions/approve-revoke';
import {
    getLightTokenBalance,
    getLightTokenDelegate,
} from './light-token-account-helpers';

featureFlags.version = VERSION.V2;

const TEST_TOKEN_DECIMALS = 9;

// ---------------------------------------------------------------------------
// Unit tests (no RPC required)
// ---------------------------------------------------------------------------

describe('createLightTokenApproveInstruction - unit', () => {
    const tokenAccount = Keypair.generate().publicKey;
    const delegate = Keypair.generate().publicKey;
    const owner = Keypair.generate().publicKey;

    it('uses LIGHT_TOKEN_PROGRAM_ID as programId', () => {
        const ix = createLightTokenApproveInstruction(
            tokenAccount,
            delegate,
            owner,
            BigInt(100),
        );
        expect(ix.programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
    });

    it('encodes discriminator byte 4', () => {
        const ix = createLightTokenApproveInstruction(
            tokenAccount,
            delegate,
            owner,
            BigInt(100),
        );
        expect(ix.data[0]).toBe(4);
    });

    it('encodes amount as u64 LE', () => {
        const amount = BigInt(1_000_000_000);
        const ix = createLightTokenApproveInstruction(
            tokenAccount,
            delegate,
            owner,
            amount,
        );
        expect(ix.data.length).toBe(9);
        const decoded = ix.data.readBigUInt64LE(1);
        expect(decoded).toBe(amount);
    });

    it('has exactly 5 account metas in correct order (owner == feePayer)', () => {
        const ix = createLightTokenApproveInstruction(
            tokenAccount,
            delegate,
            owner,
            BigInt(100),
        );
        expect(ix.keys.length).toBe(5);

        // 0: token_account - mutable, not signer
        expect(ix.keys[0].pubkey.equals(tokenAccount)).toBe(true);
        expect(ix.keys[0].isWritable).toBe(true);
        expect(ix.keys[0].isSigner).toBe(false);

        // 1: delegate - readonly, not signer
        expect(ix.keys[1].pubkey.equals(delegate)).toBe(true);
        expect(ix.keys[1].isWritable).toBe(false);
        expect(ix.keys[1].isSigner).toBe(false);

        // 2: owner - signer, writable (because owner == feePayer)
        expect(ix.keys[2].pubkey.equals(owner)).toBe(true);
        expect(ix.keys[2].isSigner).toBe(true);
        expect(ix.keys[2].isWritable).toBe(true);

        // 3: system_program - readonly
        expect(ix.keys[3].pubkey.equals(SystemProgram.programId)).toBe(true);
        expect(ix.keys[3].isWritable).toBe(false);
        expect(ix.keys[3].isSigner).toBe(false);

        // 4: fee_payer (== owner) - writable, not signer (owner already signed)
        expect(ix.keys[4].pubkey.equals(owner)).toBe(true);
        expect(ix.keys[4].isWritable).toBe(true);
        expect(ix.keys[4].isSigner).toBe(false);
    });

    it('has correct flags with separate fee payer', () => {
        const feePayer = Keypair.generate().publicKey;
        const ix = createLightTokenApproveInstruction(
            tokenAccount,
            delegate,
            owner,
            BigInt(100),
            feePayer,
        );
        expect(ix.keys.length).toBe(5);

        // 2: owner - signer, readonly (feePayer pays)
        expect(ix.keys[2].pubkey.equals(owner)).toBe(true);
        expect(ix.keys[2].isSigner).toBe(true);
        expect(ix.keys[2].isWritable).toBe(false);

        // 4: fee_payer - writable, signer
        expect(ix.keys[4].pubkey.equals(feePayer)).toBe(true);
        expect(ix.keys[4].isWritable).toBe(true);
        expect(ix.keys[4].isSigner).toBe(true);
    });
});

describe('createLightTokenRevokeInstruction - unit', () => {
    const tokenAccount = Keypair.generate().publicKey;
    const owner = Keypair.generate().publicKey;

    it('uses LIGHT_TOKEN_PROGRAM_ID as programId', () => {
        const ix = createLightTokenRevokeInstruction(tokenAccount, owner);
        expect(ix.programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
    });

    it('encodes discriminator byte 5', () => {
        const ix = createLightTokenRevokeInstruction(tokenAccount, owner);
        expect(ix.data.length).toBe(1);
        expect(ix.data[0]).toBe(5);
    });

    it('has exactly 4 account metas in correct order (owner == feePayer)', () => {
        const ix = createLightTokenRevokeInstruction(tokenAccount, owner);
        expect(ix.keys.length).toBe(4);

        // 0: token_account - mutable, not signer
        expect(ix.keys[0].pubkey.equals(tokenAccount)).toBe(true);
        expect(ix.keys[0].isWritable).toBe(true);
        expect(ix.keys[0].isSigner).toBe(false);

        // 1: owner - signer, writable (because owner == feePayer)
        expect(ix.keys[1].pubkey.equals(owner)).toBe(true);
        expect(ix.keys[1].isSigner).toBe(true);
        expect(ix.keys[1].isWritable).toBe(true);

        // 2: system_program - readonly
        expect(ix.keys[2].pubkey.equals(SystemProgram.programId)).toBe(true);
        expect(ix.keys[2].isWritable).toBe(false);
        expect(ix.keys[2].isSigner).toBe(false);

        // 3: fee_payer (== owner) - writable, not signer
        expect(ix.keys[3].pubkey.equals(owner)).toBe(true);
        expect(ix.keys[3].isWritable).toBe(true);
        expect(ix.keys[3].isSigner).toBe(false);
    });

    it('has correct flags with separate fee payer', () => {
        const feePayer = Keypair.generate().publicKey;
        const ix = createLightTokenRevokeInstruction(
            tokenAccount,
            owner,
            feePayer,
        );
        expect(ix.keys.length).toBe(4);

        // 1: owner - signer, readonly
        expect(ix.keys[1].pubkey.equals(owner)).toBe(true);
        expect(ix.keys[1].isSigner).toBe(true);
        expect(ix.keys[1].isWritable).toBe(false);

        // 3: fee_payer - writable, signer
        expect(ix.keys[3].pubkey.equals(feePayer)).toBe(true);
        expect(ix.keys[3].isWritable).toBe(true);
        expect(ix.keys[3].isSigner).toBe(true);
    });
});

// ---------------------------------------------------------------------------
// E2E tests
// ---------------------------------------------------------------------------

describe('LightToken approve/revoke - E2E', () => {
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
                undefined,
                TOKEN_PROGRAM_ID,
            )
        ).mint;

        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());
        tokenPoolInfos = await getTokenPoolInfos(rpc, mint);
    }, 60_000);

    it('should approve a delegate on a hot light-token account', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const delegate = Keypair.generate();
        const lightTokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        // Create hot ATA and mint tokens
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
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
        await loadAta(rpc, lightTokenAta, owner, mint, payer);

        // Approve delegate for 500 tokens
        const approveIx = createLightTokenApproveInstruction(
            lightTokenAta,
            delegate.publicKey,
            owner.publicKey,
            BigInt(500),
            (payer as Keypair).publicKey,
        );
        const { blockhash } = await rpc.getLatestBlockhash();
        const tx = buildAndSignTx([approveIx], payer, blockhash, [
            owner as Keypair,
        ]);
        await sendAndConfirmTx(rpc, tx);

        // Verify delegate is set
        const { delegate: actualDelegate, delegatedAmount } =
            await getLightTokenDelegate(rpc, lightTokenAta);
        expect(actualDelegate).not.toBeNull();
        expect(actualDelegate!.equals(delegate.publicKey)).toBe(true);
        expect(delegatedAmount).toBe(BigInt(500));

        // Balance unchanged
        const balance = await getLightTokenBalance(rpc, lightTokenAta);
        expect(balance).toBe(BigInt(1000));
    }, 60_000);

    it('should revoke a delegate on a hot light-token account', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const delegate = Keypair.generate();
        const lightTokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        // Setup: create ATA, mint, load, approve
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
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
        await loadAta(rpc, lightTokenAta, owner, mint, payer);

        const approveIx = createLightTokenApproveInstruction(
            lightTokenAta,
            delegate.publicKey,
            owner.publicKey,
            BigInt(500),
            (payer as Keypair).publicKey,
        );
        let { blockhash } = await rpc.getLatestBlockhash();
        let tx = buildAndSignTx([approveIx], payer, blockhash, [
            owner as Keypair,
        ]);
        await sendAndConfirmTx(rpc, tx);

        // Confirm delegate is set
        const beforeRevoke = await getLightTokenDelegate(rpc, lightTokenAta);
        expect(beforeRevoke.delegate).not.toBeNull();

        // Revoke
        const revokeIx = createLightTokenRevokeInstruction(
            lightTokenAta,
            owner.publicKey,
            (payer as Keypair).publicKey,
        );
        ({ blockhash } = await rpc.getLatestBlockhash());
        tx = buildAndSignTx([revokeIx], payer, blockhash, [owner as Keypair]);
        await sendAndConfirmTx(rpc, tx);

        // Verify delegate is cleared
        const afterRevoke = await getLightTokenDelegate(rpc, lightTokenAta);
        expect(afterRevoke.delegate).toBeNull();
        expect(afterRevoke.delegatedAmount).toBe(BigInt(0));
    }, 60_000);

    it('should overwrite delegate with a new approval', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const delegate1 = Keypair.generate();
        const delegate2 = Keypair.generate();
        const lightTokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
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
        await loadAta(rpc, lightTokenAta, owner, mint, payer);

        // Approve delegate1
        let ix = createLightTokenApproveInstruction(
            lightTokenAta,
            delegate1.publicKey,
            owner.publicKey,
            BigInt(300),
            (payer as Keypair).publicKey,
        );
        let { blockhash } = await rpc.getLatestBlockhash();
        let tx = buildAndSignTx([ix], payer, blockhash, [owner as Keypair]);
        await sendAndConfirmTx(rpc, tx);

        let info = await getLightTokenDelegate(rpc, lightTokenAta);
        expect(info.delegate!.equals(delegate1.publicKey)).toBe(true);
        expect(info.delegatedAmount).toBe(BigInt(300));

        // Overwrite with delegate2 and different amount
        ix = createLightTokenApproveInstruction(
            lightTokenAta,
            delegate2.publicKey,
            owner.publicKey,
            BigInt(700),
            (payer as Keypair).publicKey,
        );
        ({ blockhash } = await rpc.getLatestBlockhash());
        tx = buildAndSignTx([ix], payer, blockhash, [owner as Keypair]);
        await sendAndConfirmTx(rpc, tx);

        info = await getLightTokenDelegate(rpc, lightTokenAta);
        expect(info.delegate!.equals(delegate2.publicKey)).toBe(true);
        expect(info.delegatedAmount).toBe(BigInt(700));
    }, 60_000);

    it('should approve and revoke when owner is also fee payer', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const delegate = Keypair.generate();
        const lightTokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        // Create hot ATA and mint tokens (use global payer for setup)
        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
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
        await loadAta(rpc, lightTokenAta, owner, mint, payer);

        // Approve with owner as fee payer (omit feePayer param)
        const approveIx = createLightTokenApproveInstruction(
            lightTokenAta,
            delegate.publicKey,
            owner.publicKey,
            BigInt(500),
        );
        let { blockhash } = await rpc.getLatestBlockhash();
        // owner is sole signer — acts as both tx fee payer and instruction owner
        let tx = buildAndSignTx([approveIx], owner, blockhash);
        await sendAndConfirmTx(rpc, tx);

        // Verify delegate is set
        const { delegate: actualDelegate, delegatedAmount } =
            await getLightTokenDelegate(rpc, lightTokenAta);
        expect(actualDelegate).not.toBeNull();
        expect(actualDelegate!.equals(delegate.publicKey)).toBe(true);
        expect(delegatedAmount).toBe(BigInt(500));

        // Revoke with owner as fee payer
        const revokeIx = createLightTokenRevokeInstruction(
            lightTokenAta,
            owner.publicKey,
        );
        ({ blockhash } = await rpc.getLatestBlockhash());
        tx = buildAndSignTx([revokeIx], owner, blockhash);
        await sendAndConfirmTx(rpc, tx);

        // Verify delegate is cleared
        const afterRevoke = await getLightTokenDelegate(rpc, lightTokenAta);
        expect(afterRevoke.delegate).toBeNull();
        expect(afterRevoke.delegatedAmount).toBe(BigInt(0));
    }, 60_000);

    it('should fail when non-owner tries to approve', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const wrongSigner = Keypair.generate();
        const delegate = Keypair.generate();
        const lightTokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
        await mintTo(
            rpc,
            payer,
            mint,
            owner.publicKey,
            mintAuthority,
            bn(100),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );
        await loadAta(rpc, lightTokenAta, owner, mint, payer);

        // Try to approve with wrong signer
        const ix = createLightTokenApproveInstruction(
            lightTokenAta,
            delegate.publicKey,
            wrongSigner.publicKey,
            BigInt(50),
            (payer as Keypair).publicKey,
        );
        const { blockhash } = await rpc.getLatestBlockhash();
        const tx = buildAndSignTx([ix], payer, blockhash, [wrongSigner]);

        await expect(sendAndConfirmTx(rpc, tx)).rejects.toThrow();
    }, 60_000);

    it('should work with separate fee payer', async () => {
        const owner = await newAccountWithLamports(rpc, 1e9);
        const sponsor = await newAccountWithLamports(rpc, 1e9);
        const delegate = Keypair.generate();
        const lightTokenAta = getAssociatedTokenAddressInterface(
            mint,
            owner.publicKey,
        );

        await createAtaInterfaceIdempotent(rpc, payer, mint, owner.publicKey);
        await mintTo(
            rpc,
            payer,
            mint,
            owner.publicKey,
            mintAuthority,
            bn(500),
            stateTreeInfo,
            selectTokenPoolInfo(tokenPoolInfos),
        );
        await loadAta(rpc, lightTokenAta, owner, mint, payer);

        // Approve with sponsor as fee payer
        const ix = createLightTokenApproveInstruction(
            lightTokenAta,
            delegate.publicKey,
            owner.publicKey,
            BigInt(250),
            sponsor.publicKey,
        );
        const { blockhash } = await rpc.getLatestBlockhash();
        const tx = buildAndSignTx([ix], sponsor, blockhash, [
            owner as Keypair,
        ]);
        await sendAndConfirmTx(rpc, tx);

        const { delegate: actualDelegate, delegatedAmount } =
            await getLightTokenDelegate(rpc, lightTokenAta);
        expect(actualDelegate!.equals(delegate.publicKey)).toBe(true);
        expect(delegatedAmount).toBe(BigInt(250));
    }, 60_000);
});
