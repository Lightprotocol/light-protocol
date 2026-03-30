import { describe, expect, it } from 'vitest';
import { Buffer } from 'buffer';
import { Keypair, PublicKey, SystemProgram } from '@solana/web3.js';
import {
    LIGHT_TOKEN_PROGRAM_ID,
    VERSION,
    createRpc,
    featureFlags,
    newAccountWithLamports,
} from '@lightprotocol/stateless.js';
import {
    TOKEN_2022_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    createAssociatedTokenAccountIdempotentInstruction,
    createInitializeMint2Instruction,
    getAssociatedTokenAddressSync,
    unpackAccount,
} from '@solana/spl-token';
import {
    createAtaInstructions,
    createMintInstructionPlan,
    createMintInstructions,
    createMintToInstructionPlan,
    createMintToInstructions,
    createSplInterfaceInstruction,
    createTokenMetadata,
    getAta,
} from '../../src';
import { deriveSplInterfacePdaWithIndex } from '../../src/constants';
import { getMint } from '../../src/read';
import { sendInstructions } from './helpers';

featureFlags.version = VERSION.V2;

const COMPRESSED_MINT_SEED = Buffer.from('compressed_mint');

function deriveLightMintAddress(mintSigner: PublicKey): PublicKey {
    return PublicKey.findProgramAddressSync(
        [COMPRESSED_MINT_SEED, mintSigner.toBuffer()],
        LIGHT_TOKEN_PROGRAM_ID,
    )[0];
}

describe('mint and mintTo instructions', () => {
    it('creates SPL mint by default with SPL interface index 0', async () => {
        const rpc = createRpc();
        const payer = await newAccountWithLamports(rpc, 20e9);
        const keypair = Keypair.generate();
        const mintAuthority = Keypair.generate();

        const input = {
            rpc,
            payer: payer.publicKey,
            keypair,
            decimals: 9,
            mintAuthority: mintAuthority.publicKey,
        } as const;
        const plan = await createMintInstructionPlan(input);
        expect(plan).toBeDefined();

        const instructions = await createMintInstructions(input);
        await sendInstructions(rpc, payer, instructions, [keypair]);

        const mintInfo = await getMint(
            rpc,
            keypair.publicKey,
            undefined,
            TOKEN_PROGRAM_ID,
        );
        expect(mintInfo.programId.equals(TOKEN_PROGRAM_ID)).toBe(true);
        expect(mintInfo.mint.decimals).toBe(9);

        const [indexZeroPda] = deriveSplInterfacePdaWithIndex(keypair.publicKey, 0);
        expect(await rpc.getAccountInfo(indexZeroPda)).not.toBeNull();
    });

    it('creates SPL interface for an externally initialized SPL mint', async () => {
        const rpc = createRpc();
        const payer = await newAccountWithLamports(rpc, 20e9);
        const mintKeypair = Keypair.generate();
        const mintAuthority = Keypair.generate();

        const rentExemptBalance =
            await rpc.getMinimumBalanceForRentExemption(82);
        const createMintAccount = SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: mintKeypair.publicKey,
            lamports: rentExemptBalance,
            space: 82,
            programId: TOKEN_PROGRAM_ID,
        });
        const initMint = createInitializeMint2Instruction(
            mintKeypair.publicKey,
            9,
            mintAuthority.publicKey,
            null,
            TOKEN_PROGRAM_ID,
        );
        await sendInstructions(rpc, payer, [createMintAccount, initMint], [
            mintKeypair,
        ]);

        await sendInstructions(rpc, payer, [
            createSplInterfaceInstruction({
                feePayer: payer.publicKey,
                mint: mintKeypair.publicKey,
                index: 0,
                tokenProgramId: TOKEN_PROGRAM_ID,
            }),
        ]);

        const [indexZeroPda] = deriveSplInterfacePdaWithIndex(
            mintKeypair.publicKey,
            0,
        );
        expect(await rpc.getAccountInfo(indexZeroPda)).not.toBeNull();
    });

    it('creates Token-2022 mint with unified createMintInstructions flow', async () => {
        const rpc = createRpc();
        const payer = await newAccountWithLamports(rpc, 20e9);
        const keypair = Keypair.generate();
        const mintAuthority = Keypair.generate();

        const instructions = await createMintInstructions({
            rpc,
            payer: payer.publicKey,
            keypair,
            decimals: 6,
            mintAuthority: mintAuthority.publicKey,
            tokenProgramId: TOKEN_2022_PROGRAM_ID,
        });
        await sendInstructions(rpc, payer, instructions, [keypair]);

        const mintInfo = await getMint(
            rpc,
            keypair.publicKey,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );
        expect(mintInfo.programId.equals(TOKEN_2022_PROGRAM_ID)).toBe(true);
        expect(mintInfo.mint.decimals).toBe(6);
    });

    it('creates light mint with metadata using unified createMintInstructions flow', async () => {
        const rpc = createRpc();
        const payer = await newAccountWithLamports(rpc, 20e9);
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const metadata = createTokenMetadata({
            name: 'Prod Test Token',
            symbol: 'PTT',
            uri: 'https://example.com/token.json',
        });

        const input = {
            rpc,
            payer: payer.publicKey,
            keypair: mintSigner,
            decimals: 9,
            mintAuthority: mintAuthority.publicKey,
            tokenProgramId: LIGHT_TOKEN_PROGRAM_ID,
            tokenMetadata: metadata,
        } as const;
        const plan = await createMintInstructionPlan(input);
        expect(plan).toBeDefined();

        const instructions = await createMintInstructions(input);
        await sendInstructions(rpc, payer, instructions, [mintSigner, mintAuthority]);

        const lightMint = deriveLightMintAddress(mintSigner.publicKey);
        const mintInfo = await getMint(
            rpc,
            lightMint,
            undefined,
            LIGHT_TOKEN_PROGRAM_ID,
        );
        expect(mintInfo.programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
        expect(mintInfo.tokenMetadata?.name).toBe('Prod Test Token');
        expect(mintInfo.tokenMetadata?.symbol).toBe('PTT');
    });

    it('mints to SPL ATA with SPL default in createMintToInstructions', async () => {
        const rpc = createRpc();
        const payer = await newAccountWithLamports(rpc, 20e9);
        const mintKeypair = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const recipient = Keypair.generate();

        await sendInstructions(
            rpc,
            payer,
            await createMintInstructions({
                rpc,
                payer: payer.publicKey,
                keypair: mintKeypair,
                decimals: 9,
                mintAuthority: mintAuthority.publicKey,
            }),
            [mintKeypair],
        );

        const recipientAta = getAssociatedTokenAddressSync(
            mintKeypair.publicKey,
            recipient.publicKey,
            false,
            TOKEN_PROGRAM_ID,
        );
        const createRecipientAta = createAssociatedTokenAccountIdempotentInstruction(
            payer.publicKey,
            recipientAta,
            recipient.publicKey,
            mintKeypair.publicKey,
            TOKEN_PROGRAM_ID,
        );
        await sendInstructions(rpc, payer, [createRecipientAta]);

        const input = {
            mint: mintKeypair.publicKey,
            destination: recipientAta,
            authority: mintAuthority.publicKey,
            amount: 777n,
        } as const;
        const plan = await createMintToInstructionPlan(input);
        expect(plan).toBeDefined();

        const mintToIxs = await createMintToInstructions(input);
        await sendInstructions(rpc, payer, mintToIxs, [mintAuthority]);

        const accountInfo = await rpc.getAccountInfo(recipientAta);
        expect(accountInfo).not.toBeNull();
        const token = unpackAccount(recipientAta, accountInfo!, TOKEN_PROGRAM_ID);
        expect(token.amount).toBe(777n);
    });

    it('mints to Token-2022 ATA when TOKEN_2022 is selected', async () => {
        const rpc = createRpc();
        const payer = await newAccountWithLamports(rpc, 20e9);
        const mintKeypair = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const recipient = Keypair.generate();

        await sendInstructions(
            rpc,
            payer,
            await createMintInstructions({
                rpc,
                payer: payer.publicKey,
                keypair: mintKeypair,
                decimals: 6,
                mintAuthority: mintAuthority.publicKey,
                tokenProgramId: TOKEN_2022_PROGRAM_ID,
            }),
            [mintKeypair],
        );

        const recipientAta = getAssociatedTokenAddressSync(
            mintKeypair.publicKey,
            recipient.publicKey,
            false,
            TOKEN_2022_PROGRAM_ID,
        );
        const createRecipientAta = createAssociatedTokenAccountIdempotentInstruction(
            payer.publicKey,
            recipientAta,
            recipient.publicKey,
            mintKeypair.publicKey,
            TOKEN_2022_PROGRAM_ID,
        );
        await sendInstructions(rpc, payer, [createRecipientAta]);

        const mintToIxs = await createMintToInstructions({
            mint: mintKeypair.publicKey,
            destination: recipientAta,
            authority: mintAuthority.publicKey,
            amount: 555n,
            tokenProgramId: TOKEN_2022_PROGRAM_ID,
        });
        await sendInstructions(rpc, payer, mintToIxs, [mintAuthority]);

        const accountInfo = await rpc.getAccountInfo(recipientAta);
        expect(accountInfo).not.toBeNull();
        const token = unpackAccount(recipientAta, accountInfo!, TOKEN_2022_PROGRAM_ID);
        expect(token.amount).toBe(555n);
    });

    it('mints to light ATA when LIGHT program is selected', async () => {
        const rpc = createRpc();
        const payer = await newAccountWithLamports(rpc, 20e9);
        const mintSigner = Keypair.generate();
        const mintAuthority = Keypair.generate();
        const recipient = Keypair.generate();

        await sendInstructions(
            rpc,
            payer,
            await createMintInstructions({
                rpc,
                payer: payer.publicKey,
                keypair: mintSigner,
                decimals: 9,
                mintAuthority: mintAuthority.publicKey,
                tokenProgramId: LIGHT_TOKEN_PROGRAM_ID,
            }),
            [mintSigner, mintAuthority],
        );

        const lightMint = deriveLightMintAddress(mintSigner.publicKey);
        const lightAta = PublicKey.findProgramAddressSync(
            [
                recipient.publicKey.toBuffer(),
                LIGHT_TOKEN_PROGRAM_ID.toBuffer(),
                lightMint.toBuffer(),
            ],
            LIGHT_TOKEN_PROGRAM_ID,
        )[0];

        await sendInstructions(
            rpc,
            payer,
            await createAtaInstructions({
                payer: payer.publicKey,
                owner: recipient.publicKey,
                mint: lightMint,
            }),
        );

        const mintToIxs = await createMintToInstructions({
            mint: lightMint,
            destination: lightAta,
            authority: mintAuthority.publicKey,
            amount: 333n,
            tokenProgramId: LIGHT_TOKEN_PROGRAM_ID,
        });
        await sendInstructions(rpc, payer, mintToIxs, [mintAuthority]);

        const ata = await getAta({
            rpc,
            owner: recipient.publicKey,
            mint: lightMint,
        });
        expect(ata.parsed.amount).toBe(333n);
    });
});
