import { describe, expect, it } from 'vitest';
import { Keypair, SystemProgram } from '@solana/web3.js';
import { Buffer } from 'buffer';
import {
    LIGHT_TOKEN_PROGRAM_ID,
    bn,
    getDefaultAddressTreeInfo,
} from '@lightprotocol/stateless.js';
import { TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID } from '@solana/spl-token';
import {
    createApproveInstruction,
    createAtaInstruction,
    createAssociatedLightTokenAccountInstruction,
    createBurnCheckedInstruction,
    createBurnInstruction,
    createFreezeInstruction,
    createRevokeInstruction,
    createThawInstruction,
    createTransferCheckedInstruction,
    createDecompressInstruction,
    createSplInterfaceInstruction,
    createMintInstruction,
    createMintInstructions,
    createMintToInstruction,
} from '../../src/instructions';
import {
    COMPRESSED_TOKEN_PROGRAM_ID,
    deriveCpiAuthorityPda,
    deriveSplInterfacePdaWithIndex,
} from '../../src/constants';

describe('instruction builders', () => {
    function buildParsedCompressedAccount(params?: {
        amount?: bigint;
        owner?: ReturnType<typeof Keypair.generate>['publicKey'];
        mint?: ReturnType<typeof Keypair.generate>['publicKey'];
        tree?: ReturnType<typeof Keypair.generate>['publicKey'];
        queue?: ReturnType<typeof Keypair.generate>['publicKey'];
        leafIndex?: number;
    }) {
        const owner = params?.owner ?? Keypair.generate().publicKey;
        const mint = params?.mint ?? Keypair.generate().publicKey;
        const tree = params?.tree ?? Keypair.generate().publicKey;
        const queue = params?.queue ?? Keypair.generate().publicKey;
        const amount = params?.amount ?? 10n;
        const leafIndex = params?.leafIndex ?? 1;
        return {
            compressedAccount: {
                treeInfo: {
                    tree,
                    queue,
                },
                hash: new Array(32).fill(0),
                leafIndex,
                proveByIndex: false,
                owner: LIGHT_TOKEN_PROGRAM_ID,
                lamports: bn(0),
                address: null,
                data: {
                    discriminator: [2, 0, 0, 0, 0, 0, 0, 0],
                    data: Buffer.alloc(0),
                    dataHash: new Array(32).fill(0),
                },
                readOnly: false,
            },
            parsed: {
                mint,
                owner,
                amount: bn(amount.toString()),
                delegate: null,
                state: 1,
                tlv: null,
            },
        } as any;
    }

    it('creates a canonical light-token ata instruction', () => {
        const payer = Keypair.generate().publicKey;
        const owner = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;

        const instruction = createAtaInstruction({
            payer,
            owner,
            mint,
        });

        expect(instruction.programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
        expect(instruction.keys[0].pubkey.equals(owner)).toBe(true);
        expect(instruction.keys[1].pubkey.equals(mint)).toBe(true);
        expect(instruction.keys[2].pubkey.equals(payer)).toBe(true);
    });

    it('creates a checked transfer instruction', () => {
        const source = Keypair.generate().publicKey;
        const destination = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;
        const authority = Keypair.generate().publicKey;
        const payer = Keypair.generate().publicKey;

        const instruction = createTransferCheckedInstruction({
            source,
            destination,
            mint,
            authority,
            payer,
            amount: 42n,
            decimals: 9,
        });

        expect(instruction.programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
        expect(instruction.data[0]).toBe(12);
        expect(instruction.keys[0].pubkey.equals(source)).toBe(true);
        expect(instruction.keys[2].pubkey.equals(destination)).toBe(true);
    });

    it('marks fee payer as signer when transfer authority differs', () => {
        const source = Keypair.generate().publicKey;
        const destination = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;
        const authority = Keypair.generate().publicKey;
        const payer = Keypair.generate().publicKey;

        const instruction = createTransferCheckedInstruction({
            source,
            destination,
            mint,
            authority,
            payer,
            amount: 1n,
            decimals: 9,
        });

        expect(instruction.keys[3].pubkey.equals(authority)).toBe(true);
        expect(instruction.keys[3].isSigner).toBe(true);
        expect(instruction.keys[3].isWritable).toBe(false);
        expect(instruction.keys[5].pubkey.equals(payer)).toBe(true);
        expect(instruction.keys[5].isSigner).toBe(true);
        expect(instruction.keys[5].isWritable).toBe(true);
    });

    it('defaults transfer payer to authority when omitted', () => {
        const source = Keypair.generate().publicKey;
        const destination = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;
        const authority = Keypair.generate().publicKey;

        const instruction = createTransferCheckedInstruction({
            source,
            destination,
            mint,
            authority,
            amount: 1n,
            decimals: 9,
        });

        expect(instruction.keys[3].pubkey.equals(authority)).toBe(true);
        expect(instruction.keys[3].isSigner).toBe(true);
        expect(instruction.keys[3].isWritable).toBe(true);
        expect(instruction.keys[5].pubkey.equals(authority)).toBe(true);
        expect(instruction.keys[5].isSigner).toBe(false);
    });

    it('creates approve, revoke, freeze, and thaw instructions', () => {
        const tokenAccount = Keypair.generate().publicKey;
        const owner = Keypair.generate().publicKey;
        const delegate = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;
        const freezeAuthority = Keypair.generate().publicKey;

        const approve = createApproveInstruction({
            tokenAccount,
            delegate,
            owner,
            amount: 10n,
        });
        const revoke = createRevokeInstruction({
            tokenAccount,
            owner,
        });
        const freeze = createFreezeInstruction({
            tokenAccount,
            mint,
            freezeAuthority,
        });
        const thaw = createThawInstruction({
            tokenAccount,
            mint,
            freezeAuthority,
        });

        expect(approve.programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
        expect(revoke.programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
        expect(freeze.data[0]).toBe(10);
        expect(thaw.data[0]).toBe(11);
    });

    it('uses external fee payer in approve/revoke key metas', () => {
        const tokenAccount = Keypair.generate().publicKey;
        const owner = Keypair.generate().publicKey;
        const delegate = Keypair.generate().publicKey;
        const payer = Keypair.generate().publicKey;

        const approve = createApproveInstruction({
            tokenAccount,
            delegate,
            owner,
            amount: 9n,
            payer,
        });
        const revoke = createRevokeInstruction({
            tokenAccount,
            owner,
            payer,
        });

        expect(approve.keys[2].pubkey.equals(owner)).toBe(true);
        expect(approve.keys[2].isSigner).toBe(true);
        expect(approve.keys[2].isWritable).toBe(false);
        expect(approve.keys[4].pubkey.equals(payer)).toBe(true);
        expect(approve.keys[4].isSigner).toBe(true);
        expect(approve.keys[4].isWritable).toBe(true);

        expect(revoke.keys[1].pubkey.equals(owner)).toBe(true);
        expect(revoke.keys[1].isSigner).toBe(true);
        expect(revoke.keys[1].isWritable).toBe(false);
        expect(revoke.keys[3].pubkey.equals(payer)).toBe(true);
        expect(revoke.keys[3].isSigner).toBe(true);
        expect(revoke.keys[3].isWritable).toBe(true);
    });

    it('encodes burn and burn-checked discriminators and decimals', () => {
        const source = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;
        const authority = Keypair.generate().publicKey;
        const payer = Keypair.generate().publicKey;

        const burn = createBurnInstruction({
            source,
            mint,
            authority,
            amount: 123n,
            payer,
        });
        const burnChecked = createBurnCheckedInstruction({
            source,
            mint,
            authority,
            amount: 123n,
            decimals: 9,
            payer,
        });

        expect(burn.data[0]).toBe(8);
        expect(burnChecked.data[0]).toBe(15);
        expect(burnChecked.data[9]).toBe(9);
        expect(burn.keys[4].isSigner).toBe(true);
        expect(burn.keys[2].isWritable).toBe(false);
    });

    it('creates SPL ATA instruction when SPL token program is requested', () => {
        const payer = Keypair.generate().publicKey;
        const owner = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;

        const instruction = createAtaInstruction({
            payer,
            owner,
            mint,
            programId: TOKEN_PROGRAM_ID,
        });

        expect(instruction.programId.equals(TOKEN_PROGRAM_ID)).toBe(false);
        expect(instruction.keys[5].pubkey.equals(TOKEN_PROGRAM_ID)).toBe(true);
    });

    it('throws when decompress amount exceeds total input amount', () => {
        const account = buildParsedCompressedAccount({ amount: 5n });
        const owner = account.parsed.owner;
        const destination = Keypair.generate().publicKey;
        const validityProof = {
            compressedProof: null,
            rootIndices: [0],
        } as any;

        expect(() =>
            createDecompressInstruction({
                payer: owner,
                inputCompressedTokenAccounts: [account],
                toAddress: destination,
                amount: 6n,
                validityProof,
                decimals: 9,
                authority: owner,
            }),
        ).toThrow(/exceeds total input amount/i);
    });

    it('throws when decompress inputs have mixed owners', () => {
        const mint = Keypair.generate().publicKey;
        const accountA = buildParsedCompressedAccount({ mint });
        const accountB = buildParsedCompressedAccount({ mint });
        const destination = Keypair.generate().publicKey;
        const validityProof = {
            compressedProof: null,
            rootIndices: [0, 0],
        } as any;

        expect(() =>
            createDecompressInstruction({
                payer: accountA.parsed.owner,
                inputCompressedTokenAccounts: [accountA, accountB],
                toAddress: destination,
                amount: 1n,
                validityProof,
                decimals: 9,
                authority: accountA.parsed.owner,
            }),
        ).toThrow(/same owner/i);
    });

    it('defaults ATA payer to owner when omitted', () => {
        const owner = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;

        const instruction = createAtaInstruction({
            owner,
            mint,
        });

        expect(instruction.programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
        expect(instruction.keys[2].pubkey.equals(owner)).toBe(true);
        expect(instruction.keys[2].isSigner).toBe(true);
    });

    it('omits light-token config/rent keys when compressible config is null', () => {
        const feePayer = Keypair.generate().publicKey;
        const owner = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;

        const instruction = createAssociatedLightTokenAccountInstruction({
            feePayer,
            owner,
            mint,
            compressibleConfig: null,
        });

        expect(instruction.programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
        expect(instruction.keys).toHaveLength(5);
    });

    it('defaults associated light-token fee payer to owner when omitted', () => {
        const owner = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;

        const instruction = createAssociatedLightTokenAccountInstruction({
            owner,
            mint,
        });

        expect(instruction.programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
        expect(instruction.keys[2].pubkey.equals(owner)).toBe(true);
        expect(instruction.keys[2].isSigner).toBe(true);
    });

    it('creates spl interface instruction with required index', () => {
        const feePayer = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;
        const index = 0;

        const instruction = createSplInterfaceInstruction({
            feePayer,
            mint,
            index,
        });

        const [splInterfacePda] = deriveSplInterfacePdaWithIndex(mint, index);

        expect(instruction.programId.equals(COMPRESSED_TOKEN_PROGRAM_ID)).toBe(
            true,
        );
        expect(
            instruction.data.equals(
                Buffer.from([23, 169, 27, 122, 147, 169, 209, 152]),
            ),
        ).toBe(true);
        expect(instruction.keys).toHaveLength(6);
        expect(instruction.keys[0].pubkey.equals(feePayer)).toBe(true);
        expect(instruction.keys[0].isSigner).toBe(true);
        expect(instruction.keys[0].isWritable).toBe(true);
        expect(instruction.keys[1].pubkey.equals(splInterfacePda)).toBe(true);
        expect(instruction.keys[1].isWritable).toBe(true);
        expect(instruction.keys[2].pubkey.equals(SystemProgram.programId)).toBe(
            true,
        );
        expect(instruction.keys[3].pubkey.equals(mint)).toBe(true);
        expect(instruction.keys[3].isWritable).toBe(true);
        expect(instruction.keys[4].pubkey.equals(TOKEN_PROGRAM_ID)).toBe(true);
        expect(instruction.keys[5].pubkey.equals(deriveCpiAuthorityPda())).toBe(
            true,
        );
    });

    it('creates spl interface instruction with custom token program', () => {
        const feePayer = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;
        const index = 1;

        const instruction = createSplInterfaceInstruction({
            feePayer,
            mint,
            index,
            tokenProgramId: TOKEN_2022_PROGRAM_ID,
        });

        expect(
            instruction.keys[4].pubkey.equals(TOKEN_2022_PROGRAM_ID),
        ).toBe(true);
    });

    it('throws when spl interface index is out of range', () => {
        const feePayer = Keypair.generate().publicKey;
        const mint = Keypair.generate().publicKey;

        expect(() =>
            createSplInterfaceInstruction({
                feePayer,
                mint,
                index: 256,
            }),
        ).toThrow(/integer in \[0, 255\]/i);
    });

    it('creates initialize mint instruction for SPL/T22', () => {
        const mint = Keypair.generate().publicKey;
        const mintAuthority = Keypair.generate().publicKey;
        const freezeAuthority = Keypair.generate().publicKey;

        const instruction = createMintInstruction({
            mint,
            decimals: 9,
            mintAuthority,
            freezeAuthority,
            tokenProgramId: TOKEN_2022_PROGRAM_ID,
        });

        expect(instruction.programId.equals(TOKEN_2022_PROGRAM_ID)).toBe(true);
        expect(instruction.keys[0].pubkey.equals(mint)).toBe(true);
        expect(instruction.keys[0].isWritable).toBe(true);
    });

    it('creates light-token mint-to with optional fee payer and maxTopUp', () => {
        const mint = Keypair.generate().publicKey;
        const destination = Keypair.generate().publicKey;
        const authority = Keypair.generate().publicKey;
        const payer = Keypair.generate().publicKey;

        const instruction = createMintToInstruction({
            mint,
            destination,
            authority,
            amount: 123n,
            payer,
            tokenProgramId: LIGHT_TOKEN_PROGRAM_ID,
            maxTopUp: 10,
        });

        expect(instruction.programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
        expect(instruction.data[0]).toBe(7);
        expect(instruction.data.length).toBe(11);
        expect(instruction.keys).toHaveLength(5);
        expect(instruction.keys[4].pubkey.equals(payer)).toBe(true);
        expect(instruction.keys[4].isSigner).toBe(true);
        expect(instruction.keys[4].isWritable).toBe(true);
    });

    it('creates spl mint-to when SPL token program is requested', () => {
        const mint = Keypair.generate().publicKey;
        const destination = Keypair.generate().publicKey;
        const authority = Keypair.generate().publicKey;

        const instruction = createMintToInstruction({
            mint,
            destination,
            authority,
            amount: 50n,
            tokenProgramId: TOKEN_PROGRAM_ID,
        });

        expect(instruction.programId.equals(TOKEN_PROGRAM_ID)).toBe(true);
        expect(instruction.keys).toHaveLength(3);
    });

    it('defaults mint-to instruction program to SPL', () => {
        const mint = Keypair.generate().publicKey;
        const destination = Keypair.generate().publicKey;
        const authority = Keypair.generate().publicKey;

        const instruction = createMintToInstruction({
            mint,
            destination,
            authority,
            amount: 1n,
        });

        expect(instruction.programId.equals(TOKEN_PROGRAM_ID)).toBe(true);
    });

    it('defaults createMintInstructions to SPL flow', async () => {
        const keypair = Keypair.generate().publicKey;
        const payer = Keypair.generate().publicKey;
        const mintAuthority = Keypair.generate().publicKey;
        const rpc = {
            getMinimumBalanceForRentExemption: async () => 1_461_600,
        } as any;

        const instructions = await createMintInstructions({
            rpc,
            payer,
            keypair,
            decimals: 9,
            mintAuthority,
        });

        expect(instructions).toHaveLength(3);
        expect(instructions[1].programId.equals(TOKEN_PROGRAM_ID)).toBe(true);
    });

    it('creates light mint flow when LIGHT program is requested', async () => {
        const mintSigner = Keypair.generate().publicKey;
        const payer = Keypair.generate().publicKey;
        const mintAuthority = Keypair.generate().publicKey;
        const freezeAuthority = Keypair.generate().publicKey;
        const defaultAddressTreeInfo = getDefaultAddressTreeInfo();
        const outputStateTreeInfo = {
            queue: Keypair.generate().publicKey,
        } as any;
        const rpc = {
            getValidityProofV2: async () => ({
                compressedProof: null,
                rootIndices: [0],
            }),
        } as any;

        const instructions = await createMintInstructions({
            rpc,
            payer,
            keypair: mintSigner,
            decimals: 9,
            mintAuthority,
            freezeAuthority,
            tokenProgramId: LIGHT_TOKEN_PROGRAM_ID,
            addressTreeInfo: defaultAddressTreeInfo,
            outputStateTreeInfo,
        });

        expect(instructions).toHaveLength(1);
        expect(instructions[0].programId.equals(LIGHT_TOKEN_PROGRAM_ID)).toBe(
            true,
        );
        expect(instructions[0].data[0]).toBe(103);
    });
});
