import {
    PublicKey,
    Keypair,
    TransactionInstruction,
    SystemProgram,
    Connection,
} from '@solana/web3.js';
import { BN, Program, AnchorProvider, setProvider } from '@coral-xyz/anchor';
import { IDL, LightCompressedToken } from './idl/light_compressed_token';
import {
    CompressedProof,
    LightSystemProgram,
    ParsedTokenAccount,
    TokenTransferOutputData,
    bn,
    confirmConfig,
    CompressedTokenInstructionDataTransfer,
    defaultStaticAccountsStruct,
    sumUpLamports,
    toArray,
    useWallet,
    validateSameOwner,
    validateSufficientBalance,
    defaultTestStateTreeAccounts,
} from '@lightprotocol/stateless.js';
import {
    MINT_SIZE,
    TOKEN_PROGRAM_ID,
    createInitializeMint2Instruction,
    createMintToInstruction,
} from '@solana/spl-token';
import { CPI_AUTHORITY_SEED, POOL_SEED } from './constants';
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
     * The state tree that the tx output should be inserted into. Defaults to a
     * public state tree if unspecified.
     */
    outputStateTree?: PublicKey;
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
     * The state tree that the change tx output should be inserted into.
     * Defaults to a public state tree if unspecified.
     */
    outputStateTree?: PublicKey;
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

/**
 * Create Mint account for compressed Tokens
 */
export type CreateMintParams = {
    /**
     * Tx feepayer
     */
    feePayer: PublicKey;
    /**
     * Mint authority
     */
    authority: PublicKey;
    /**
     * Mint public key
     */
    mint: PublicKey;
    /**
     * Mint decimals
     */
    decimals: number;
    /**
     * Optional: freeze authority
     */
    freezeAuthority: PublicKey | null;
    /**
     * lamport amount for mint account rent exemption
     */
    rentExemptBalance: number;
};

/**
 * Create compressed token accounts
 */
export type MintToParams = {
    /**
     * Tx feepayer
     */
    feePayer: PublicKey;
    /**
     * Mint authority
     */
    authority: PublicKey;
    /**
     * Mint public key
     */
    mint: PublicKey;
    /**
     * The Solana Public Keys to mint to.
     */
    toPubkey: PublicKey[] | PublicKey;
    /**
     * The amount of compressed tokens to mint.
     */
    amount: BN | BN[] | number | number[];
    /**
     * Public key of the state tree to mint into. Defaults to a public state
     * tree if unspecified.
     */
    merkleTree?: PublicKey;
};

/**
 * Register an existing SPL mint account to the compressed token program
 * Creates an omnibus account for the mint
 */
export type RegisterMintParams = {
    /** Tx feepayer */
    feePayer: PublicKey;
    /** Mint public key */
    mint: PublicKey;
};

/**
 * Mint from existing SPL mint to compressed token accounts
 */
export type ApproveAndMintToParams = {
    /**
     * Tx feepayer
     */
    feePayer: PublicKey;
    /**
     * Mint authority
     */
    authority: PublicKey;
    /**
     * Mint authority (associated) token account
     */
    authorityTokenAccount: PublicKey;
    /**
     * Mint public key
     */
    mint: PublicKey;
    /**
     * The Solana Public Key to mint to.
     */
    toPubkey: PublicKey;
    /**
     * The amount of compressed tokens to mint.
     */
    amount: BN | number;
    /**
     * Public key of the state tree to mint into. Defaults to a public state
     * tree if unspecified.
     */
    merkleTree?: PublicKey;
};

/**
 * Sum up the token amounts of the compressed token accounts
 */
export const sumUpTokenAmount = (accounts: ParsedTokenAccount[]): BN => {
    return accounts.reduce(
        (acc, account: ParsedTokenAccount) => acc.add(account.parsed.amount),
        bn(0),
    );
};

/**
 * Validate that all the compressed token accounts are owned by the same owner.
 */
export const validateSameTokenOwner = (accounts: ParsedTokenAccount[]) => {
    const owner = accounts[0].parsed.owner;
    accounts.forEach(acc => {
        if (!acc.parsed.owner.equals(owner)) {
            throw new Error('Token accounts must be owned by the same owner');
        }
    });
};

/**
 * Parse compressed token accounts to get the mint, current owner and delegate.
 */
export const parseTokenData = (
    compressedTokenAccounts: ParsedTokenAccount[],
) => {
    const mint = compressedTokenAccounts[0].parsed.mint;
    const currentOwner = compressedTokenAccounts[0].parsed.owner;
    const delegate = compressedTokenAccounts[0].parsed.delegate;

    return { mint, currentOwner, delegate };
};

/**
 * Create the output state for a transfer transaction.
 * @param inputCompressedTokenAccounts  Input state
 * @param toAddress                     Recipient address
 * @param amount                        Amount of tokens to transfer
 * @returns                             Output token data for the transfer
 *                                      instruction
 */
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
                tlv: null,
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
            tlv: null,
        },
        {
            owner: toAddress,
            amount,
            lamports: bn(0),
            tlv: null,
        },
    ];
    return outputCompressedAccounts;
}

/**
 * Create the output state for a compress transaction.
 * @param inputCompressedTokenAccounts  Input state
 * @param amount                        Amount of tokens to compress
 * @returns                             Output token data for the compress
 *                                      instruction
 */
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
            tlv: null,
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
        'HXVfQ44ATEi9WBKLSCCwM54KokdkzqXci9xCQ7ST9SYN',
    );

    private static _program: Program<LightCompressedToken> | null = null;

    /** @internal */
    static get program(): Program<LightCompressedToken> {
        if (!this._program) {
            this.initializeProgram();
        }
        return this._program!;
    }

    /**
     * @internal
     * Initializes the program statically if not already initialized.
     */
    private static initializeProgram() {
        if (!this._program) {
            /// Note: We can use a mock connection because we're using the
            /// program only for serde and building instructions, not for
            /// interacting with the network.
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
            [CPI_AUTHORITY_SEED],
            this.programId,
        );
        return address;
    }

    /**
     * Construct createMint instruction for compressed tokens
     */
    static async createMint(
        params: CreateMintParams,
    ): Promise<TransactionInstruction[]> {
        const { mint, authority, feePayer, rentExemptBalance } = params;

        /// Create and initialize SPL Mint account
        const createMintAccountInstruction = SystemProgram.createAccount({
            fromPubkey: feePayer,
            lamports: rentExemptBalance,
            newAccountPubkey: mint,
            programId: TOKEN_PROGRAM_ID,
            space: MINT_SIZE,
        });

        const initializeMintInstruction = createInitializeMint2Instruction(
            mint,
            params.decimals,
            authority,
            params.freezeAuthority,
            TOKEN_PROGRAM_ID,
        );

        const ix = await this.createTokenPool({
            feePayer,
            mint,
        });

        return [createMintAccountInstruction, initializeMintInstruction, ix];
    }

    /**
     * Enable compression for an existing SPL mint, creating an omnibus account.
     * For new mints, use `CompressedTokenProgram.createMint`.
     */
    static async createTokenPool(
        params: RegisterMintParams,
    ): Promise<TransactionInstruction> {
        const { mint, feePayer } = params;

        const tokenPoolPda = this.deriveTokenPoolPda(mint);

        const ix = await this.program.methods
            .createTokenPool()
            .accounts({
                mint,
                feePayer,
                tokenPoolPda,
                systemProgram: SystemProgram.programId,
                tokenProgram: TOKEN_PROGRAM_ID,
                cpiAuthorityPda: this.deriveCpiAuthorityPda,
            })
            .instruction();

        return ix;
    }

    /**
     * Construct mintTo instruction for compressed tokens
     */
    static async mintTo(params: MintToParams): Promise<TransactionInstruction> {
        const systemKeys = defaultStaticAccountsStruct();

        const { mint, feePayer, authority, merkleTree, toPubkey, amount } =
            params;

        const tokenPoolPda = this.deriveTokenPoolPda(mint);

        const amounts = toArray<BN | number>(amount).map(amount => bn(amount));

        const toPubkeys = toArray(toPubkey);
        const instruction = await this.program.methods
            .mintTo(toPubkeys, amounts, null)
            .accounts({
                feePayer,
                authority,
                cpiAuthorityPda: this.deriveCpiAuthorityPda,
                mint,
                tokenPoolPda,
                tokenProgram: TOKEN_PROGRAM_ID,
                lightSystemProgram: LightSystemProgram.programId,
                registeredProgramPda: systemKeys.registeredProgramPda,
                noopProgram: systemKeys.noopProgram,
                accountCompressionAuthority:
                    systemKeys.accountCompressionAuthority,
                accountCompressionProgram: systemKeys.accountCompressionProgram,
                merkleTree:
                    merkleTree ?? defaultTestStateTreeAccounts().merkleTree,
                selfProgram: this.programId,
                solPoolPda: null,
            })
            .instruction();
        return instruction;
    }

    /// TODO: add compressBatch functionality for batch minting
    /**
     * Mint tokens from registed SPL mint account to a compressed account
     */
    static async approveAndMintTo(params: ApproveAndMintToParams) {
        const {
            mint,
            feePayer,
            authorityTokenAccount,
            authority,
            merkleTree,
            toPubkey,
        } = params;

        const amount: bigint = BigInt(params.amount.toString());

        /// 1. Mint to existing ATA of mintAuthority.
        const splMintToInstruction = createMintToInstruction(
            mint,
            authorityTokenAccount,
            authority,
            amount,
        );

        /// 2. Compress from mint authority ATA to recipient compressed account
        const compressInstruction = await this.compress({
            payer: feePayer,
            owner: authority,
            source: authorityTokenAccount,
            toAddress: toPubkey,
            mint,
            amount: params.amount,
            outputStateTree: merkleTree,
        });

        return [splMintToInstruction, compressInstruction];
    }
    /**
     * Construct transfer instruction for compressed tokens
     */
    static async transfer(
        params: TransferParams,
    ): Promise<TransactionInstruction> {
        const {
            payer,
            inputCompressedTokenAccounts,
            recentInputStateRootIndices,
            recentValidityProof,
            amount,
            outputStateTrees,
            toAddress,
        } = params;

        const tokenTransferOutputs: TokenTransferOutputData[] =
            createTransferOutputState(
                inputCompressedTokenAccounts,
                toAddress,
                amount,
            );
        const {
            inputTokenDataWithContext,
            packedOutputTokenData,
            remainingAccountMetas,
        } = packCompressedTokenAccounts({
            inputCompressedTokenAccounts,
            outputStateTrees,
            rootIndices: recentInputStateRootIndices,
            tokenTransferOutputs,
        });

        const { mint, currentOwner } = parseTokenData(
            inputCompressedTokenAccounts,
        );

        const data: CompressedTokenInstructionDataTransfer = {
            proof: recentValidityProof,
            mint,
            delegatedTransfer: null, // TODO: implement
            inputTokenDataWithContext,
            outputCompressedAccounts: packedOutputTokenData,
            compressOrDecompressAmount: null,
            isCompress: false,
            cpiContext: null,
            lamportsChangeAccountMerkleTreeIndex: null,
        };

        const encodedData = this.program.coder.types.encode(
            'CompressedTokenInstructionDataTransfer',
            data,
        );

        const {
            accountCompressionAuthority,
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
                lightSystemProgram: LightSystemProgram.programId,
                registeredProgramPda: registeredProgramPda,
                noopProgram: noopProgram,
                accountCompressionAuthority: accountCompressionAuthority,
                accountCompressionProgram: accountCompressionProgram,
                selfProgram: this.programId,
                tokenPoolPda: null,
                compressOrDecompressTokenAccount: null,
                tokenProgram: null,
            })
            .remainingAccounts(remainingAccountMetas)
            .instruction();

        return instruction;
    }

    /**
     * Construct compress instruction
     * @returns compressInstruction
     */
    static async compress(
        params: CompressParams,
    ): Promise<TransactionInstruction> {
        const { payer, owner, source, toAddress, mint, outputStateTree } =
            params;
        const amount = bn(params.amount);

        const tokenTransferOutputs: TokenTransferOutputData[] = [
            {
                owner: toAddress,
                amount,
                lamports: bn(0),
                tlv: null,
            },
        ];
        const {
            inputTokenDataWithContext,
            packedOutputTokenData,
            remainingAccountMetas,
        } = packCompressedTokenAccounts({
            inputCompressedTokenAccounts: [],
            outputStateTrees: outputStateTree,
            rootIndices: [],
            tokenTransferOutputs,
        });

        const data: CompressedTokenInstructionDataTransfer = {
            proof: null,
            mint,
            delegatedTransfer: null, // TODO: implement
            inputTokenDataWithContext,
            outputCompressedAccounts: packedOutputTokenData,
            compressOrDecompressAmount: amount,
            isCompress: true,
            cpiContext: null,
            lamportsChangeAccountMerkleTreeIndex: null,
        };

        const encodedData = this.program.coder.types.encode(
            'CompressedTokenInstructionDataTransfer',
            data,
        );

        const {
            accountCompressionAuthority,
            noopProgram,
            registeredProgramPda,
            accountCompressionProgram,
        } = defaultStaticAccountsStruct();

        const instruction = await this.program.methods
            .transfer(encodedData)
            .accounts({
                feePayer: payer,
                authority: owner,
                cpiAuthorityPda: this.deriveCpiAuthorityPda,
                lightSystemProgram: LightSystemProgram.programId,
                registeredProgramPda: registeredProgramPda,
                noopProgram: noopProgram,
                accountCompressionAuthority: accountCompressionAuthority,
                accountCompressionProgram: accountCompressionProgram,
                selfProgram: this.programId,
                tokenPoolPda: this.deriveTokenPoolPda(mint),
                compressOrDecompressTokenAccount: source, // token
                tokenProgram: TOKEN_PROGRAM_ID,
            })
            .remainingAccounts(remainingAccountMetas)
            .instruction();

        return instruction;
    }

    /**
     * Construct decompress instruction
     */
    static async decompress(
        params: DecompressParams,
    ): Promise<TransactionInstruction> {
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
            packedOutputTokenData,
            remainingAccountMetas,
        } = packCompressedTokenAccounts({
            inputCompressedTokenAccounts,
            outputStateTrees: outputStateTree,
            rootIndices: recentInputStateRootIndices,
            tokenTransferOutputs: tokenTransferOutputs,
        });

        const { mint, currentOwner } = parseTokenData(
            inputCompressedTokenAccounts,
        );

        const data: CompressedTokenInstructionDataTransfer = {
            proof: recentValidityProof,
            mint,
            delegatedTransfer: null, // TODO: implement
            inputTokenDataWithContext,
            outputCompressedAccounts: packedOutputTokenData,
            compressOrDecompressAmount: amount,
            isCompress: false,
            cpiContext: null,
            lamportsChangeAccountMerkleTreeIndex: null,
        };

        const encodedData = this.program.coder.types.encode(
            'CompressedTokenInstructionDataTransfer',
            data,
        );

        const {
            accountCompressionAuthority,
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
                lightSystemProgram: LightSystemProgram.programId,
                registeredProgramPda: registeredProgramPda,
                noopProgram: noopProgram,
                accountCompressionAuthority: accountCompressionAuthority,
                accountCompressionProgram: accountCompressionProgram,
                selfProgram: this.programId,
                tokenPoolPda: this.deriveTokenPoolPda(mint),
                compressOrDecompressTokenAccount: toAddress,
                tokenProgram: TOKEN_PROGRAM_ID,
            })
            .remainingAccounts(remainingAccountMetas)
            .instruction();

        return instruction;
    }
}
