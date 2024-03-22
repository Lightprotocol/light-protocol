import {
    PublicKey,
    Keypair,
    TransactionInstruction,
    SystemProgram,
    Connection,
} from '@solana/web3.js';
import { BN, Program, AnchorProvider, setProvider } from '@coral-xyz/anchor';
import { IDL, PspCompressedToken } from './idl/psp_compressed_token';
import {
    LightSystemProgram,
    PublicTransactionEvent_IdlType,
    bn,
    confirmConfig,
    defaultStaticAccountsStruct,
    getConnection,
    toArray,
    useWallet,
} from '@lightprotocol/stateless.js';
import {
    MINT_SIZE,
    TOKEN_PROGRAM_ID,
    createInitializeMint2Instruction,
} from '@solana/spl-token';
import { MINT_AUTHORITY_SEED, POOL_SEED } from './constants';
import { Buffer } from 'buffer';

/** Create Mint account for compressed Tokens */
export type CreateMintParams = {
    /** Tx feepayer */
    feePayer: PublicKey;
    /** Mint authority */
    authority: PublicKey;
    /** Mint public key */
    mint: PublicKey;
    /** Mint decimals */
    decimals: number;
    /** Optional: freeze authority */
    freezeAuthority: PublicKey | null;
    /** lamport amount for mint account rent exemption */
    rentExemptBalance: number;
};

/**
 * Create compressed token accounts
 */
export type MintToParams = {
    /** Tx feepayer */
    feePayer: PublicKey;
    /** Mint authority */
    authority: PublicKey;
    /** Mint public key */
    mint: PublicKey;
    /** The Solana Public Keys to mint to. Accepts batches */
    toPubkey: PublicKey[] | PublicKey;
    /** The amount of compressed tokens to mint. Accepts batches */
    amount: BN | BN[] | number | number[]; // TODO: check if considers mint decimals
    /** Public key of the state tree to mint into. */
    merkleTree: PublicKey; // TODO: make optional with default system state trees
};

export class CompressedTokenProgram {
    /**
     * @internal
     */
    constructor() {}

    /**
     * Public key that identifies the CompressedPda program
     */
    static programId: PublicKey = new PublicKey(
        // TODO: can add check to ensure its consistent with the idl
        '9sixVEthz2kMSKfeApZXHwuboT6DZuT6crAYJTciUCqE',
    );

    private static _program: Program<PspCompressedToken> | null = null;

    static get program(): Program<PspCompressedToken> {
        if (!this._program) {
            this.initializeProgram();
        }
        return this._program!;
    }

    /**
     * Initializes the program statically if not already initialized.
     */
    private static initializeProgram() {
        if (!this._program) {
            /// We can use a mock connection because we're using the program only for
            /// serde and building instructions, not for interacting with the network.
            const mockKeypair = Keypair.generate();
            const mockConnection = new Connection(
                'http://127.0.0.1:8899',
                'confirmed',
            );
            const mockProvider = new AnchorProvider(
                mockConnection,
                useWallet(mockKeypair),
                confirmConfig,
            );
            setProvider(mockProvider);
            this._program = new Program(IDL, this.programId, mockProvider);
        }
    }

    /** @internal */
    static deriveMintAuthorityPda = (
        authority: PublicKey,
        mint: PublicKey,
    ): PublicKey => {
        const [pubkey] = PublicKey.findProgramAddressSync(
            [MINT_AUTHORITY_SEED, authority.toBuffer(), mint.toBuffer()],
            this.programId,
        );
        return pubkey;
    };

    /** @internal */
    static deriveTokenPoolPda(mint: PublicKey): PublicKey {
        const seeds = [POOL_SEED, mint.toBuffer()];
        const [address, _] = PublicKey.findProgramAddressSync(
            seeds,
            this.programId,
        );
        return address;
    }

    /** @internal */
    static get deriveCpiAuthorityPda(): PublicKey {
        const [address, _] = PublicKey.findProgramAddressSync(
            [Buffer.from('cpi_authority')],
            this.programId,
        );
        return address;
    }

    static async createMint(
        params: CreateMintParams,
    ): Promise<TransactionInstruction[]> {
        const { mint, authority, feePayer, rentExemptBalance } = params;

        const createMintAccountInstruction = SystemProgram.createAccount({
            fromPubkey: feePayer,
            lamports: rentExemptBalance,
            newAccountPubkey: mint,
            programId: TOKEN_PROGRAM_ID,
            space: MINT_SIZE,
        });

        const mintAuthorityPda = this.deriveMintAuthorityPda(authority, mint);

        const initializeMintInstruction = createInitializeMint2Instruction(
            mint,
            params.decimals,
            mintAuthorityPda,
            params.freezeAuthority,
            TOKEN_PROGRAM_ID,
        );

        const fundAuthorityPdaInstruction = SystemProgram.transfer({
            fromPubkey: feePayer,
            toPubkey: mintAuthorityPda,
            lamports: rentExemptBalance, // TODO: check that this is the right PDA size
        });

        const tokenPoolPda = this.deriveTokenPoolPda(mint);

        const ix = await this.program.methods
            .createMint()
            .accounts({
                mint,
                feePayer,
                authority,
                tokenPoolPda,
                systemProgram: SystemProgram.programId,
                mintAuthorityPda,
                tokenProgram: TOKEN_PROGRAM_ID,
            })
            .instruction();

        return [
            createMintAccountInstruction,
            initializeMintInstruction,
            fundAuthorityPdaInstruction,
            ix,
        ];
    }

    static async mintTo(params: MintToParams): Promise<TransactionInstruction> {
        const systemKeys = defaultStaticAccountsStruct();

        const { mint, feePayer, authority, merkleTree, toPubkey, amount } =
            params;

        const tokenPoolPda = this.deriveTokenPoolPda(mint);
        const mintAuthorityPda = this.deriveMintAuthorityPda(authority, mint);

        const amounts = toArray<BN | number>(amount).map(amount => bn(amount));

        const toPubkeys = toArray(toPubkey);

        const ix = await this.program.methods
            .mintTo(toPubkeys, amounts)
            .accounts({
                feePayer,
                authority,
                mintAuthorityPda,
                mint,
                tokenPoolPda,
                tokenProgram: TOKEN_PROGRAM_ID,
                compressedPdaProgram: LightSystemProgram.programId,
                registeredProgramPda: systemKeys.registeredProgramPda,
                noopProgram: systemKeys.noopProgram,
                pspAccountCompressionAuthority:
                    systemKeys.pspAccountCompressionAuthority,
                accountCompressionProgram: systemKeys.accountCompressionProgram,
                merkleTree,
            })
            .instruction();

        return ix;
    }
}

// TODO: move to serde
if (import.meta.vitest) {
    const { it, describe } = import.meta.vitest;

    describe('Program serde', () => {
        it('should decode token layout from tlvDataElement correctly', () => {
            const tlvDataElementData = Buffer.from([
                17, 83, 159, 197, 140, 93, 111, 210, 204, 87, 177, 176, 53, 172,
                85, 246, 188, 121, 104, 73, 239, 121, 154, 117, 79, 42, 29, 89,
                206, 227, 91, 128, 24, 91, 116, 158, 172, 135, 49, 150, 5, 204,
                228, 125, 131, 190, 235, 131, 166, 185, 57, 57, 224, 221, 10,
                123, 83, 145, 148, 227, 216, 1, 152, 52, 100, 0, 0, 0, 0, 0, 0,
                0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]);
            const decoded = CompressedTokenProgram.program.coder.types.decode(
                'TokenTlvDataClient',
                tlvDataElementData,
            );

            console.log(decoded);
        });
        it("should decode 'PublicTransactionEvent' from data correctly + TokenTlvDataClient", () => {
            const PublicTransactionEventData = Buffer.from([
                1, 0, 0, 0, 131, 219, 249, 246, 221, 196, 33, 3, 114, 23, 121,
                235, 18, 229, 71, 152, 39, 87, 169, 208, 143, 101, 43, 128, 245,
                59, 22, 134, 182, 231, 116, 33, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0,
                0, 131, 219, 249, 246, 221, 196, 33, 3, 114, 23, 121, 235, 18,
                229, 71, 152, 39, 87, 169, 208, 143, 101, 43, 128, 245, 59, 22,
                134, 182, 231, 116, 33, 83, 0, 0, 0, 253, 8, 131, 238, 132, 68,
                108, 9, 175, 149, 239, 98, 98, 100, 222, 135, 45, 145, 67, 70,
                180, 90, 75, 78, 176, 94, 84, 95, 56, 201, 60, 81, 104, 97, 96,
                185, 122, 159, 102, 107, 255, 171, 120, 14, 49, 197, 211, 242,
                230, 80, 231, 137, 157, 16, 152, 34, 209, 185, 112, 209, 138,
                221, 228, 196, 142, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 29, 116, 78, 174, 219, 0, 27, 92, 172, 2, 150, 253, 27,
                8, 61, 7, 110, 205, 2, 142, 203, 5, 238, 223, 253, 25, 217, 122,
                255, 198, 67, 132, 2, 0, 0, 0, 131, 219, 249, 246, 221, 196, 33,
                3, 114, 23, 121, 235, 18, 229, 71, 152, 39, 87, 169, 208, 143,
                101, 43, 128, 245, 59, 22, 134, 182, 231, 116, 33, 3, 23, 116,
                190, 161, 85, 183, 105, 2, 210, 96, 171, 251, 35, 230, 70, 184,
                162, 76, 17, 34, 148, 163, 126, 54, 92, 38, 29, 25, 135, 147,
                44, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 2, 0, 0, 0, 0, 0,
                0, 0, 131, 219, 249, 246, 221, 196, 33, 3, 114, 23, 121, 235,
                18, 229, 71, 152, 39, 87, 169, 208, 143, 101, 43, 128, 245, 59,
                22, 134, 182, 231, 116, 33, 83, 0, 0, 0, 253, 8, 131, 238, 132,
                68, 108, 9, 175, 149, 239, 98, 98, 100, 222, 135, 45, 145, 67,
                70, 180, 90, 75, 78, 176, 94, 84, 95, 56, 201, 60, 81, 207, 201,
                5, 158, 72, 15, 156, 23, 190, 249, 217, 79, 28, 52, 227, 223,
                204, 3, 239, 114, 202, 116, 116, 228, 75, 182, 167, 177, 149,
                198, 91, 61, 100, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 26, 30, 215, 158, 64, 253, 113, 55, 94, 36, 73, 181,
                186, 0, 133, 58, 183, 17, 183, 75, 209, 66, 43, 66, 136, 0, 28,
                173, 140, 228, 74, 107, 131, 219, 249, 246, 221, 196, 33, 3,
                114, 23, 121, 235, 18, 229, 71, 152, 39, 87, 169, 208, 143, 101,
                43, 128, 245, 59, 22, 134, 182, 231, 116, 33, 35, 178, 4, 102,
                17, 21, 168, 136, 157, 56, 50, 108, 154, 228, 155, 34, 142, 84,
                254, 22, 231, 149, 148, 193, 110, 114, 111, 4, 137, 183, 203,
                239, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 2, 0, 0, 0, 0, 0,
                0, 0, 131, 219, 249, 246, 221, 196, 33, 3, 114, 23, 121, 235,
                18, 229, 71, 152, 39, 87, 169, 208, 143, 101, 43, 128, 245, 59,
                22, 134, 182, 231, 116, 33, 83, 0, 0, 0, 253, 8, 131, 238, 132,
                68, 108, 9, 175, 149, 239, 98, 98, 100, 222, 135, 45, 145, 67,
                70, 180, 90, 75, 78, 176, 94, 84, 95, 56, 201, 60, 81, 104, 97,
                96, 185, 122, 159, 102, 107, 255, 171, 120, 14, 49, 197, 211,
                242, 230, 80, 231, 137, 157, 16, 152, 34, 209, 185, 112, 209,
                138, 221, 228, 196, 42, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 42, 20, 77, 234, 38, 120, 221, 63, 182, 191, 57,
                41, 202, 139, 186, 3, 55, 156, 87, 103, 16, 118, 232, 32, 135,
                223, 214, 228, 248, 78, 204, 182, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]);

            const decoded: PublicTransactionEvent_IdlType =
                CompressedTokenProgram.program.coder.types.decode(
                    'PublicTransactionEvent',
                    PublicTransactionEventData,
                );
            const decodedTok =
                CompressedTokenProgram.program.coder.types.decode(
                    'TokenTlvDataClient',
                    Buffer.from(decoded.outUtxos[0].data!.tlvElements[0].data),
                );
            console.log('DECODED', decodedTok);
        });
    });
}
