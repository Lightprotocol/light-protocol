import {
    PublicKey,
    Keypair,
    TransactionInstruction,
    SystemProgram,
    Connection,
    ComputeBudgetProgram,
} from '@solana/web3.js';
import { BN, Program, AnchorProvider, setProvider } from '@coral-xyz/anchor';
import { IDL, PspCompressedToken } from './idl/psp_compressed_token';
import {
    CompressedProof,
    CompressedTokenInstructionDataTransfer,
    LightSystemProgram,
    ParsedTokenAccount,
    TokenTransferOutputData,
    accountCompressionProgram,
    bn,
    confirmConfig,
    defaultStaticAccountsStruct,
    sumUpLamports,
    toArray,
    useWallet,
    validateSameOwner,
    validateSufficientBalance,
} from '@lightprotocol/stateless.js';
import {
    MINT_SIZE,
    TOKEN_PROGRAM_ID,
    approve,
    createApproveCheckedInstruction,
    createApproveInstruction,
    createInitializeMint2Instruction,
} from '@solana/spl-token';
import {
    CPI_AUTHORITY_SEED,
    MINT_AUTHORITY_SEED,
    POOL_SEED,
} from './constants';
import { Buffer } from 'buffer';
import { packCompressedTokenAccounts } from './instructions/pack-compressed-token-accounts';

type CompressParams = {
    /**
     * The payer of the transaction.
     */
    payer: PublicKey;
    /**
     * owner of the *uncompressed* token account.
     */
    owner: PublicKey;
    /**
     * source (associated) token account address.
     */
    source: PublicKey;
    /**
     * owner of the compressed token account.
     */
    toAddress: PublicKey;
    /**
     * Mint address of the token to compress.
     */
    mint: PublicKey;
    /**
     * amount of tokens to compress.
     */
    amount: number | BN;
    /**
     * The state tree that the tx output should be inserted into.
     */
    outputStateTree: PublicKey;
};

type DecompressParams = {
    /**
     * The payer of the transaction.
     */
    payer: PublicKey;
    /**
     * input state to be consumed
     */
    inputCompressedTokenAccounts: ParsedTokenAccount[];
    /**
     * address of **uncompressed** destination token account.
     */
    toAddress: PublicKey;
    /**
     * amount of tokens to decompress.
     */
    amount: number | BN;
    /**
     * The state tree that the change tx output should be inserted into.
     */
    outputStateTree: PublicKey;
    /**
     * The recent state root indices of the input state. The expiry is tied to
     * the proof.
     */
    recentInputStateRootIndices: number[];
    /**
     * The recent validity proof for state inclusion of the input state. It
     * expires after n slots.
     */
    recentValidityProof: CompressedProof;
};

export type TransferParams = {
    /**
     * The payer of the transaction
     */
    payer: PublicKey;
    /**
     * The input state to be consumed
     */
    inputCompressedTokenAccounts: ParsedTokenAccount[];
    /**
     * Recipient address
     */
    toAddress: PublicKey;
    /**
     * Amount of tokens to transfer
     */
    amount: BN | number;
    /**
     * The recent state root indices of the input state. The expiry is tied to
     * the proof.

     */
    recentInputStateRootIndices: number[];
    /**
     * The recent validity proof for state inclusion of the input state. It
     * expires after n slots.
     */
    recentValidityProof: CompressedProof;
    /**
     * The state trees that the tx output should be inserted into. This can be a
     * single PublicKey or an array of PublicKey. Defaults to the 0th state tree
     * of input state.
     */
    outputStateTrees?: PublicKey[] | PublicKey;
};

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

export const sumUpTokenAmount = (accounts: ParsedTokenAccount[]): BN => {
    return accounts.reduce(
        (acc, account: ParsedTokenAccount) => acc.add(account.parsed.amount),
        bn(0),
    );
};
export const validateSameTokenOwner = (accounts: ParsedTokenAccount[]) => {
    const owner = accounts[0].parsed.owner;
    accounts.forEach(acc => {
        if (!acc.parsed.owner.equals(owner)) {
            throw new Error('Token accounts must be owned by the same owner');
        }
    });
};

export const parseInputTokenData = (
    inputCompressedTokenAccounts: ParsedTokenAccount[],
) => {
    const mint = inputCompressedTokenAccounts[0].parsed.mint;
    const currentOwner = inputCompressedTokenAccounts[0].parsed.owner;
    const delegate = inputCompressedTokenAccounts[0].parsed.delegate;

    return { mint, currentOwner, delegate };
};

export function createTransferOutputState(
    inputCompressedTokenAccounts: ParsedTokenAccount[],
    toAddress: PublicKey,
    amount: number | BN,
): TokenTransferOutputData[] {
    amount = bn(amount);
    const inputAmount = sumUpTokenAmount(inputCompressedTokenAccounts);
    const inputLamports = sumUpLamports(
        inputCompressedTokenAccounts.map(acc => acc.compressedAccount),
    );

    const changeAmount = inputAmount.sub(amount);

    validateSufficientBalance(changeAmount);

    if (changeAmount.eq(bn(0)) && inputLamports.eq(bn(0))) {
        return [
            {
                owner: toAddress,
                amount,
                lamports: inputLamports,
            },
        ];
    }

    /// validates token program
    validateSameOwner(
        inputCompressedTokenAccounts.map(acc => acc.compressedAccount),
    );
    validateSameTokenOwner(inputCompressedTokenAccounts);

    const outputCompressedAccounts: TokenTransferOutputData[] = [
        {
            owner: inputCompressedTokenAccounts[0].parsed.owner,
            amount: changeAmount,
            lamports: inputLamports,
        },
        {
            owner: toAddress,
            amount,
            lamports: bn(0),
        },
    ];
    return outputCompressedAccounts;
}

export function createDecompressOutputState(
    inputCompressedTokenAccounts: ParsedTokenAccount[],
    amount: number | BN,
): TokenTransferOutputData[] {
    amount = bn(amount);
    const inputLamports = sumUpLamports(
        inputCompressedTokenAccounts.map(acc => acc.compressedAccount),
    );
    const inputAmount = sumUpTokenAmount(inputCompressedTokenAccounts);
    const changeAmount = inputAmount.sub(amount);

    validateSufficientBalance(changeAmount);

    /// lamports gets decompressed
    if (changeAmount.eq(bn(0)) && inputLamports.eq(bn(0))) {
        return [];
    }

    validateSameOwner(
        inputCompressedTokenAccounts.map(acc => acc.compressedAccount),
    );
    validateSameTokenOwner(inputCompressedTokenAccounts);

    const tokenTransferOutputs: TokenTransferOutputData[] = [
        {
            owner: inputCompressedTokenAccounts[0].parsed.owner,
            amount: changeAmount,
            lamports: inputLamports,
        },
    ];
    return tokenTransferOutputs;
}

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
            [
                CPI_AUTHORITY_SEED,
                defaultStaticAccountsStruct().accountCompressionProgram.toBuffer(),
            ],
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
                cpiAuthorityPda: this.deriveCpiAuthorityPda,
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

    static async transfer(params: TransferParams) {
        const {
            payer,
            inputCompressedTokenAccounts,
            recentInputStateRootIndices,
            recentValidityProof,
            amount,
            outputStateTrees,
            toAddress,
        } = params;

        const outputCompressedAccounts: TokenTransferOutputData[] =
            createTransferOutputState(
                inputCompressedTokenAccounts,
                toAddress,
                amount,
            );

        /// Pack
        const {
            inputTokenDataWithContext,
            outputStateMerkleTreeIndices,
            remainingAccountMetas,
            pubkeyArray,
        } = packCompressedTokenAccounts({
            inputCompressedTokenAccounts,
            outputCompressedAccountsLength: outputCompressedAccounts.length,
            outputStateTrees,
        });

        const { mint, currentOwner } = parseInputTokenData(
            inputCompressedTokenAccounts,
        );

        const data: CompressedTokenInstructionDataTransfer = {
            proof: recentValidityProof,
            rootIndices: recentInputStateRootIndices,
            mint,
            signerIsDelegate: false, // TODO: implement
            inputTokenDataWithContext,
            outputCompressedAccounts,
            outputStateMerkleTreeAccountIndices: Buffer.from(
                outputStateMerkleTreeIndices,
            ),
            pubkeyArray,
            compressionAmount: null,
            isCompress: false,
        };

        const encodedData = this.program.coder.types.encode(
            'CompressedTokenInstructionDataTransfer',
            data,
        );

        const {
            pspAccountCompressionAuthority,
            noopProgram,
            registeredProgramPda,
            accountCompressionProgram,
        } = defaultStaticAccountsStruct();

        const instruction = await this.program.methods
            .transfer(encodedData)
            .accounts({
                feePayer: payer!,
                authority: currentOwner!,
                cpiAuthorityPda: this.deriveCpiAuthorityPda,
                compressedPdaProgram: LightSystemProgram.programId,
                registeredProgramPda: registeredProgramPda,
                noopProgram: noopProgram,
                pspAccountCompressionAuthority: pspAccountCompressionAuthority,
                accountCompressionProgram: accountCompressionProgram,
                selfProgram: this.programId,
                tokenPoolPda: null,
                decompressTokenAccount: null,
                tokenProgram: null,
            })
            .remainingAccounts(remainingAccountMetas)
            .instruction();

        return [
            ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }),
            instruction,
        ];
    }

    static async compress(params: CompressParams) {
        const { payer, owner, source, toAddress, mint, outputStateTree } =
            params;
        const amount = bn(params.amount);

        const outputCompressedAccounts: TokenTransferOutputData[] = [
            {
                owner: toAddress,
                amount,
                lamports: bn(0),
            },
        ];

        /// Pack
        const {
            inputTokenDataWithContext,
            outputStateMerkleTreeIndices,
            remainingAccountMetas,
            pubkeyArray,
        } = packCompressedTokenAccounts({
            inputCompressedTokenAccounts: [],
            outputCompressedAccountsLength: outputCompressedAccounts.length,
            outputStateTrees: [outputStateTree],
        });

        const data: CompressedTokenInstructionDataTransfer = {
            proof: null,
            rootIndices: [],
            mint,
            signerIsDelegate: false, // TODO: implement
            inputTokenDataWithContext,
            outputCompressedAccounts,
            outputStateMerkleTreeAccountIndices: Buffer.from(
                outputStateMerkleTreeIndices,
            ),
            pubkeyArray,
            compressionAmount: amount,
            isCompress: true,
        };

        const encodedData = this.program.coder.types.encode(
            'CompressedTokenInstructionDataTransfer',
            data,
        );

        const {
            pspAccountCompressionAuthority,
            noopProgram,
            registeredProgramPda,
            accountCompressionProgram,
        } = defaultStaticAccountsStruct();

        /// TODO: add support for multiSigners
        /// TODO: validate that we don't need approveChecked
        const approveInstruction = createApproveInstruction(
            source,
            this.deriveCpiAuthorityPda,
            owner,
            BigInt(amount.toString()),
        );

        const instruction = await this.program.methods
            .transfer(encodedData)
            .accounts({
                feePayer: payer,
                authority: owner,
                cpiAuthorityPda: this.deriveCpiAuthorityPda,
                compressedPdaProgram: LightSystemProgram.programId,
                registeredProgramPda: registeredProgramPda,
                noopProgram: noopProgram,
                pspAccountCompressionAuthority: pspAccountCompressionAuthority,
                accountCompressionProgram: accountCompressionProgram,
                selfProgram: this.programId,
                tokenPoolPda: this.deriveTokenPoolPda(mint),
                decompressTokenAccount: source, // token
                tokenProgram: TOKEN_PROGRAM_ID,
            })
            .remainingAccounts(remainingAccountMetas)
            .instruction();

        return [
            ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }),
            approveInstruction,
            instruction,
        ];
    }

    static async decompress(params: DecompressParams) {
        const {
            payer,
            inputCompressedTokenAccounts,
            toAddress,
            outputStateTree,
            recentValidityProof,
            recentInputStateRootIndices,
        } = params;
        const amount = bn(params.amount);

        const tokenTransferOutputs = createDecompressOutputState(
            inputCompressedTokenAccounts,
            amount,
        );

        /// Pack
        const {
            inputTokenDataWithContext,
            outputStateMerkleTreeIndices,
            remainingAccountMetas,
            pubkeyArray,
        } = packCompressedTokenAccounts({
            inputCompressedTokenAccounts,
            outputCompressedAccountsLength: tokenTransferOutputs.length,
            outputStateTrees: [outputStateTree],
        });
        const { mint, currentOwner } = parseInputTokenData(
            inputCompressedTokenAccounts,
        );

        const data: CompressedTokenInstructionDataTransfer = {
            proof: recentValidityProof,
            rootIndices: recentInputStateRootIndices,
            mint,
            signerIsDelegate: false, // TODO: implement
            inputTokenDataWithContext,
            outputCompressedAccounts: tokenTransferOutputs,
            outputStateMerkleTreeAccountIndices: Buffer.from(
                outputStateMerkleTreeIndices,
            ),
            pubkeyArray,
            compressionAmount: amount,
            isCompress: false,
        };

        const encodedData = this.program.coder.types.encode(
            'CompressedTokenInstructionDataTransfer',
            data,
        );

        const {
            pspAccountCompressionAuthority,
            noopProgram,
            registeredProgramPda,
            accountCompressionProgram,
        } = defaultStaticAccountsStruct();

        const instruction = await this.program.methods
            .transfer(encodedData)
            .accounts({
                feePayer: payer,
                authority: currentOwner,
                cpiAuthorityPda: this.deriveCpiAuthorityPda,
                compressedPdaProgram: LightSystemProgram.programId,
                registeredProgramPda: registeredProgramPda,
                noopProgram: noopProgram,
                pspAccountCompressionAuthority: pspAccountCompressionAuthority,
                accountCompressionProgram: accountCompressionProgram,
                selfProgram: this.programId,
                tokenPoolPda: this.deriveTokenPoolPda(mint),
                decompressTokenAccount: toAddress, // token
                tokenProgram: TOKEN_PROGRAM_ID,
            })
            .remainingAccounts(remainingAccountMetas)
            .instruction();

        return [
            ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }),
            instruction,
        ];
    }
}
