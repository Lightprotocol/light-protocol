import {
    PublicKey,
    TransactionInstruction,
    SystemProgram,
    Connection,
    AddressLookupTableProgram,
    AccountMeta,
    ComputeBudgetProgram,
} from '@solana/web3.js';
import BN from 'bn.js';
import { Buffer } from 'buffer';
import {
    ValidityProof,
    LightSystemProgram,
    ParsedTokenAccount,
    bn,
    defaultStaticAccountsStruct,
    sumUpLamports,
    toArray,
    validateSameOwner,
    validateSufficientBalance,
    defaultTestStateTreeAccounts,
    TreeInfo,
    CompressedProof,
    featureFlags,
    TreeType,
} from '@lightprotocol/stateless.js';
import {
    MINT_SIZE,
    TOKEN_2022_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
    createInitializeMint2Instruction,
    createMintToInstruction,
} from '@solana/spl-token';
import {
    CPI_AUTHORITY_SEED,
    POOL_SEED,
    CREATE_TOKEN_POOL_DISCRIMINATOR,
    ADD_TOKEN_POOL_DISCRIMINATOR,
} from './constants';
import { checkMint, packCompressedTokenAccounts } from './utils';
import {
    encodeTransferInstructionData,
    encodeCompressSplTokenAccountInstructionData,
    encodeMintToInstructionData,
    createTokenPoolAccountsLayout,
    mintToAccountsLayout,
    transferAccountsLayout,
    approveAccountsLayout,
    revokeAccountsLayout,
    encodeApproveInstructionData,
    encodeRevokeInstructionData,
    addTokenPoolAccountsLayout,
    encodeBatchCompressInstructionData,
} from './layout';
import {
    BatchCompressInstructionData,
    CompressedTokenInstructionDataApprove,
    CompressedTokenInstructionDataRevoke,
    CompressedTokenInstructionDataTransfer,
    DelegatedTransfer,
    TokenTransferOutputData,
} from './types';
import {
    checkTokenPoolInfo,
    TokenPoolInfo,
} from './utils/get-token-pool-infos';

export type CompressParams = {
    /**
     * Fee payer
     */
    payer: PublicKey;
    /**
     * Owner of uncompressed token account
     */
    owner: PublicKey;
    /**
     * Source SPL Token account address
     */
    source: PublicKey;
    /**
     * Recipient address(es)
     */
    toAddress: PublicKey | PublicKey[];
    /**
     * Token amount(s) to compress
     */
    amount: number | BN | number[] | BN[];
    /**
     * SPL Token mint address
     */
    mint: PublicKey;
    /**
     * State tree to write to
     */
    outputStateTreeInfo: TreeInfo;
    /**
     * Token pool
     */
    tokenPoolInfo: TokenPoolInfo;
};

export type CompressSplTokenAccountParams = {
    /**
     * Fee payer
     */
    feePayer: PublicKey;
    /**
     * SPL Token account owner
     */
    authority: PublicKey;
    /**
     * SPL Token account to compress
     */
    tokenAccount: PublicKey;
    /**
     * SPL Token mint address
     */
    mint: PublicKey;
    /**
     * Amount to leave in token account
     */
    remainingAmount?: BN;
    /**
     * State tree to write to
     */
    outputStateTreeInfo: TreeInfo;
    /**
     * Token pool
     */
    tokenPoolInfo: TokenPoolInfo;
};

export type DecompressParams = {
    /**
     * Fee payer
     */
    payer: PublicKey;
    /**
     * Source compressed token accounts
     */
    inputCompressedTokenAccounts: ParsedTokenAccount[];
    /**
     * Destination uncompressed token account
     */
    toAddress: PublicKey;
    /**
     * Token amount to decompress
     */
    amount: number | BN;
    /**
     * Validity proof for input state
     */
    recentValidityProof: ValidityProof | CompressedProof | null;
    /**
     * Recent state root indices
     */
    recentInputStateRootIndices: number[];
    /**
     * Token pool(s)
     */
    tokenPoolInfos: TokenPoolInfo | TokenPoolInfo[];
};

export type TransferParams = {
    /**
     * Fee payer
     */
    payer: PublicKey;
    /**
     * Source compressed token accounts
     */
    inputCompressedTokenAccounts: ParsedTokenAccount[];
    /**
     * Recipient address
     */
    toAddress: PublicKey;
    /**
     * Token amount to transfer
     */
    amount: BN | number;
    /**
     * Validity proof for input state
     */
    recentValidityProof: ValidityProof | CompressedProof | null;
    /**
     * Recent state root indices
     */
    recentInputStateRootIndices: number[];
};

export type ApproveParams = {
    /**
     * Fee payer
     */
    payer: PublicKey;
    /**
     * Source compressed token accounts
     */
    inputCompressedTokenAccounts: ParsedTokenAccount[];
    /**
     * Recipient address
     */
    toAddress: PublicKey;
    /**
     * Token amount to approve
     */
    amount: BN | number;
    /**
     * Validity proof for input state
     */
    recentValidityProof: ValidityProof | CompressedProof | null;
    /**
     * Recent state root indices
     */
    recentInputStateRootIndices: number[];
};

export type RevokeParams = {
    /**
     * Fee payer
     */
    payer: PublicKey;
    /**
     * Input compressed token accounts
     */
    inputCompressedTokenAccounts: ParsedTokenAccount[];
    /**
     * Validity proof for input state
     */
    recentValidityProof: ValidityProof | CompressedProof | null;
    /**
     * Recent state root indices
     */
    recentInputStateRootIndices: number[];
};

/**
 * Create Mint account for compressed Tokens
 */
export type CreateMintParams = {
    /**
     * Fee payer
     */
    feePayer: PublicKey;
    /**
     * SPL Mint address
     */
    mint: PublicKey;
    /**
     * Mint authority
     */
    authority: PublicKey;
    /**
     * Optional: freeze authority
     */
    freezeAuthority: PublicKey | null;
    /**
     * Mint decimals
     */
    decimals: number;
    /**
     * lamport amount for mint account rent exemption
     */
    rentExemptBalance: number;
    /**
     * Optional: The token program ID. Default: SPL Token Program ID
     */
    tokenProgramId?: PublicKey;
    /**
     * Optional: Mint size to use, defaults to MINT_SIZE
     */
    mintSize?: number;
};

/**
 * Parameters for merging compressed token accounts
 */
export type MergeTokenAccountsParams = {
    /**
     * Fee payer
     */
    payer: PublicKey;
    /**
     * Owner of the compressed token accounts to be merged
     */
    owner: PublicKey;
    /**
     * SPL Token mint address
     */
    mint: PublicKey;
    /**
     * Array of compressed token accounts to merge
     */
    inputCompressedTokenAccounts: ParsedTokenAccount[];
    /**
     * Validity proof for state inclusion
     */
    recentValidityProof: ValidityProof | CompressedProof | null;
    /**
     * State root indices of the input state
     */
    recentInputStateRootIndices: number[];
};

/**
 * Create compressed token accounts
 */
export type MintToParams = {
    /**
     * Fee payer
     */
    feePayer: PublicKey;
    /**
     * Token mint address
     */
    mint: PublicKey;
    /**
     * Mint authority
     */
    authority: PublicKey;
    /**
     * Recipient address(es)
     */
    toPubkey: PublicKey[] | PublicKey;
    /**
     * Token amount(s) to mint
     */
    amount: BN | BN[] | number | number[];
    /**
     * State tree for minted tokens
     */
    outputStateTreeInfo: TreeInfo;
    /**
     * Token pool
     */
    tokenPoolInfo: TokenPoolInfo;
};

/**
 * Register an existing SPL mint account to the compressed token program
 * Creates an omnibus account for the mint
 */
export type CreateTokenPoolParams = {
    /**
     * Fee payer
     */
    feePayer: PublicKey;
    /**
     * SPL Mint address
     */
    mint: PublicKey;
    /**
     * Optional: The token program ID. Default: SPL Token Program ID
     */
    tokenProgramId?: PublicKey;
};

export type AddTokenPoolParams = {
    /**
     * Fee payer
     */
    feePayer: PublicKey;
    /**
     * Token mint address
     */
    mint: PublicKey;
    /**
     * Token pool index
     */
    poolIndex: number;
    /**
     * Optional: Token program ID. Default: SPL Token Program ID
     */
    tokenProgramId?: PublicKey;
};

/**
 * Mint from existing SPL mint to compressed token accounts
 */
export type ApproveAndMintToParams = {
    /**
     * Fee payer
     */
    feePayer: PublicKey;
    /**
     * SPL Mint address
     */
    mint: PublicKey;
    /**
     * Mint authority
     */
    authority: PublicKey;
    /**
     * Mint authority (associated) token account
     */
    authorityTokenAccount: PublicKey;
    /**
     * Recipient address
     */
    toPubkey: PublicKey;
    /**
     * Token amount to mint
     */
    amount: BN | number;
    /**
     * State tree to write to
     */
    outputStateTreeInfo: TreeInfo;
    /**
     * Token pool
     */
    tokenPoolInfo: TokenPoolInfo;
};

export type CreateTokenProgramLookupTableParams = {
    /**
     * Fee payer
     */
    payer: PublicKey;
    /**
     * Authority of the transaction
     */
    authority: PublicKey;
    /**
     * Optional Mint addresses to store in the lookup table
     */
    mints?: PublicKey[];
    /**
     * Recently finalized Solana slot
     */
    recentSlot: number;
    /**
     * Optional additional addresses to store in the lookup table
     */
    remainingAccounts?: PublicKey[];
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

export const parseMaybeDelegatedTransfer = (
    inputs: ParsedTokenAccount[],
    outputs: TokenTransferOutputData[],
): { delegatedTransfer: DelegatedTransfer | null; authority: PublicKey } => {
    if (inputs.length < 1)
        throw new Error('Must supply at least one input token account.');

    const owner = inputs[0].parsed.owner;

    const delegatedAccountsIndex = inputs.findIndex(a => a.parsed.delegate);

    /// Fast path: no delegated account used
    if (delegatedAccountsIndex === -1)
        return { delegatedTransfer: null, authority: owner };

    const delegate = inputs[delegatedAccountsIndex].parsed.delegate;
    const delegateChangeAccountIndex = outputs.length <= 1 ? null : 0;

    return {
        delegatedTransfer: {
            owner,
            delegateChangeAccountIndex,
        },
        authority: delegate!,
    };
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
        'cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m',
    );

    /**
     * Set a custom programId via PublicKey or base58 encoded string.
     * This method is not required for regular usage.
     *
     * Use this only if you know what you are doing.
     */
    static setProgramId(programId: PublicKey | string) {
        this.programId =
            typeof programId === 'string'
                ? new PublicKey(programId)
                : programId;
    }

    /**
     * Derive the token pool pda.
     * To derive the token pool pda with bump, use {@link deriveTokenPoolPdaWithIndex}.
     *
     * @param mint The mint of the token pool
     *
     * @returns The token pool pda
     */
    static deriveTokenPoolPda(mint: PublicKey): PublicKey {
        const seeds = [POOL_SEED, mint.toBuffer()];
        const [address, _] = PublicKey.findProgramAddressSync(
            seeds,
            this.programId,
        );
        return address;
    }

    /**
     * Find the index and bump for a given token pool pda and mint.
     *
     * @param poolPda The token pool pda to find the index and bump for
     * @param mint The mint of the token pool
     *
     * @returns The index and bump number.
     */
    static findTokenPoolIndexAndBump(
        poolPda: PublicKey,
        mint: PublicKey,
    ): [number, number] {
        for (let index = 0; index < 5; index++) {
            const derivedPda =
                CompressedTokenProgram.deriveTokenPoolPdaWithIndex(mint, index);
            if (derivedPda[0].equals(poolPda)) {
                return [index, derivedPda[1]];
            }
        }
        throw new Error('Token pool not found');
    }

    /**
     * Derive the token pool pda with index.
     *
     * @param mint The mint of the token pool
     * @param index Index. starts at 0. The Protocol supports 4 indexes aka token pools
     * per mint.
     *
     * @returns The token pool pda and bump.
     */
    static deriveTokenPoolPdaWithIndex(
        mint: PublicKey,
        index: number,
    ): [PublicKey, number] {
        let seeds: Buffer[] = [];
        if (index === 0) {
            seeds = [Buffer.from('pool'), mint.toBuffer(), Buffer.from([])]; // legacy, 1st
        } else {
            seeds = [
                Buffer.from('pool'),
                mint.toBuffer(),
                Buffer.from([index]),
            ];
        }
        const [address, bump] = PublicKey.findProgramAddressSync(
            seeds,
            this.programId,
        );
        return [address, bump];
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
     * Construct createMint instruction for compressed tokens.
     *
     * @param feePayer              Fee payer.
     * @param mint                  SPL Mint address.
     * @param authority             Mint authority.
     * @param freezeAuthority       Optional: freeze authority.
     * @param decimals              Decimals.
     * @param rentExemptBalance     Lamport amount for mint account rent exemption.
     * @param tokenProgramId        Optional: Token program ID. Default: SPL Token Program ID
     * @param mintSize              Optional: mint size. Default: MINT_SIZE
     *
     * @returns [createMintAccountInstruction, initializeMintInstruction,
     * createTokenPoolInstruction]
     *
     * Note that `createTokenPoolInstruction` must be executed after
     * `initializeMintInstruction`.
     */
    static async createMint({
        feePayer,
        mint,
        authority,
        freezeAuthority,
        decimals,
        rentExemptBalance,
        tokenProgramId,
        mintSize,
    }: CreateMintParams): Promise<TransactionInstruction[]> {
        const tokenProgram = tokenProgramId ?? TOKEN_PROGRAM_ID;

        /// Create and initialize SPL Mint account
        const createMintAccountInstruction = SystemProgram.createAccount({
            fromPubkey: feePayer,
            lamports: rentExemptBalance,
            newAccountPubkey: mint,
            programId: tokenProgram,
            space: mintSize ?? MINT_SIZE,
        });

        const initializeMintInstruction = createInitializeMint2Instruction(
            mint,
            decimals,
            authority,
            freezeAuthority,
            tokenProgram,
        );

        const createTokenPoolInstruction = await this.createTokenPool({
            feePayer,
            mint,
            tokenProgramId: tokenProgram,
        });

        return [
            createMintAccountInstruction,
            initializeMintInstruction,
            createTokenPoolInstruction,
        ];
    }

    /**
     * Enable compression for an existing SPL mint, creating an omnibus account.
     * For new mints, use `CompressedTokenProgram.createMint`.
     *
     * @param feePayer              Fee payer.
     * @param mint                  SPL Mint address.
     * @param tokenProgramId        Optional: Token program ID. Default: SPL
     *                              Token Program ID
     *
     * @returns The createTokenPool instruction
     */
    static async createTokenPool({
        feePayer,
        mint,
        tokenProgramId,
    }: CreateTokenPoolParams): Promise<TransactionInstruction> {
        const tokenProgram = tokenProgramId ?? TOKEN_PROGRAM_ID;

        const tokenPoolPda = this.deriveTokenPoolPdaWithIndex(mint, 0);

        const keys = createTokenPoolAccountsLayout({
            mint,
            feePayer,
            tokenPoolPda: tokenPoolPda[0],
            tokenProgram,
            cpiAuthorityPda: this.deriveCpiAuthorityPda,
            systemProgram: SystemProgram.programId,
        });

        return new TransactionInstruction({
            programId: this.programId,
            keys,
            data: CREATE_TOKEN_POOL_DISCRIMINATOR,
        });
    }

    /**
     * Add a token pool to an existing SPL mint.  For new mints, use
     * {@link createTokenPool}.
     *
     * @param feePayer              Fee payer.
     * @param mint                  SPL Mint address.
     * @param poolIndex             Pool index.
     * @param tokenProgramId        Optional: Token program ID. Default: SPL
     *                              Token Program ID
     *
     * @returns The addTokenPool instruction
     */
    static async addTokenPool({
        feePayer,
        mint,
        poolIndex,
        tokenProgramId,
    }: AddTokenPoolParams): Promise<TransactionInstruction> {
        if (poolIndex <= 0) {
            throw new Error(
                'Pool index must be greater than 0. For 0, use CreateTokenPool instead.',
            );
        }
        if (poolIndex > 3) {
            throw new Error(
                `Invalid poolIndex ${poolIndex}. Max 4 pools per mint.`,
            );
        }

        const tokenProgram = tokenProgramId ?? TOKEN_PROGRAM_ID;

        const existingTokenPoolPda = this.deriveTokenPoolPdaWithIndex(
            mint,
            poolIndex - 1,
        );
        const tokenPoolPda = this.deriveTokenPoolPdaWithIndex(mint, poolIndex);

        const keys = addTokenPoolAccountsLayout({
            mint,
            feePayer,
            tokenPoolPda: tokenPoolPda[0],
            existingTokenPoolPda: existingTokenPoolPda[0],
            tokenProgram,
            cpiAuthorityPda: this.deriveCpiAuthorityPda,
            systemProgram: SystemProgram.programId,
        });

        return new TransactionInstruction({
            programId: this.programId,
            keys,
            data: Buffer.concat([
                new Uint8Array(ADD_TOKEN_POOL_DISCRIMINATOR),
                new Uint8Array(Buffer.from([poolIndex])),
            ]),
        });
    }

    /**
     * Construct mintTo instruction for compressed tokens
     *
     * @param feePayer              Fee payer.
     * @param mint                  SPL Mint address.
     * @param authority             Mint authority.
     * @param toPubkey              Recipient owner address.
     * @param amount                Amount of tokens to mint.
     * @param outputStateTreeInfo   State tree to write to.
     * @param tokenPoolInfo         Token pool info.
     *
     * @returns The mintTo instruction
     */
    static async mintTo({
        feePayer,
        mint,
        authority,
        toPubkey,
        amount,
        outputStateTreeInfo,
        tokenPoolInfo,
    }: MintToParams): Promise<TransactionInstruction> {
        const systemKeys = defaultStaticAccountsStruct();
        const tokenProgram = tokenPoolInfo.tokenProgram;
        checkTokenPoolInfo(tokenPoolInfo, mint);

        const amounts = toArray<BN | number>(amount).map(amount => bn(amount));
        const toPubkeys = toArray(toPubkey);

        if (amounts.length !== toPubkeys.length) {
            throw new Error(
                'Amount and toPubkey arrays must have the same length',
            );
        }

        const keys = mintToAccountsLayout({
            mint,
            feePayer,
            authority,
            cpiAuthorityPda: this.deriveCpiAuthorityPda,
            tokenProgram,
            tokenPoolPda: tokenPoolInfo.tokenPoolPda,
            lightSystemProgram: LightSystemProgram.programId,
            registeredProgramPda: systemKeys.registeredProgramPda,
            noopProgram: systemKeys.noopProgram,
            accountCompressionAuthority: systemKeys.accountCompressionAuthority,
            accountCompressionProgram: systemKeys.accountCompressionProgram,
            merkleTree:
                outputStateTreeInfo.treeType === TreeType.StateV2
                    ? outputStateTreeInfo.queue
                    : outputStateTreeInfo.tree,
            selfProgram: this.programId,
            systemProgram: SystemProgram.programId,
            solPoolPda: null,
        });

        const data = encodeMintToInstructionData({
            recipients: toPubkeys,
            amounts,
            lamports: null,
        });

        return new TransactionInstruction({
            programId: this.programId,
            keys,
            data,
        });
    }

    /**
     * Mint tokens from registered SPL mint account to a compressed account
     *
     * @param feePayer              Fee payer.
     * @param mint                  SPL Mint address.
     * @param authority             Mint authority.
     * @param authorityTokenAccount The mint authority's associated token
     *                              account (ATA).
     * @param toPubkey              Recipient owner address.
     * @param amount                Amount of tokens to mint.
     * @param outputStateTreeInfo   State tree to write to.
     * @param tokenPoolInfo         Token pool info.
     *
     * @returns The mintTo instruction
     */
    static async approveAndMintTo({
        feePayer,
        mint,
        authority,
        authorityTokenAccount,
        toPubkey,
        amount,
        outputStateTreeInfo,
        tokenPoolInfo,
    }: ApproveAndMintToParams) {
        const amountBigInt: bigint = BigInt(amount.toString());

        /// 1. Mint to existing ATA of mintAuthority.
        const splMintToInstruction = createMintToInstruction(
            mint,
            authorityTokenAccount,
            authority,
            amountBigInt,
            [],
            tokenPoolInfo.tokenProgram,
        );

        /// 2. Compress from mint authority ATA to recipient compressed account
        const compressInstruction = await this.compress({
            payer: feePayer,
            owner: authority,
            source: authorityTokenAccount,
            toAddress: toPubkey,
            mint,
            amount,
            outputStateTreeInfo,
            tokenPoolInfo,
        });

        return [splMintToInstruction, compressInstruction];
    }

    /**
     * Construct transfer instruction for compressed tokens
     *
     * @param payer                         Fee payer.
     * @param inputCompressedTokenAccounts  Source compressed token accounts.
     * @param toAddress                     Recipient owner address.
     * @param amount                        Amount of tokens to transfer.
     * @param recentValidityProof           Recent validity proof.
     * @param recentInputStateRootIndices   Recent state root indices.
     *
     * @returns The transfer instruction
     */
    static async transfer({
        payer,
        inputCompressedTokenAccounts,
        toAddress,
        amount,
        recentValidityProof,
        recentInputStateRootIndices,
    }: TransferParams): Promise<TransactionInstruction> {
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
            rootIndices: recentInputStateRootIndices,
            tokenTransferOutputs,
        });

        const { mint } = parseTokenData(inputCompressedTokenAccounts);

        const { delegatedTransfer, authority } = parseMaybeDelegatedTransfer(
            inputCompressedTokenAccounts,
            tokenTransferOutputs,
        );

        const rawData: CompressedTokenInstructionDataTransfer = {
            proof: recentValidityProof,
            mint,
            delegatedTransfer,
            inputTokenDataWithContext,
            outputCompressedAccounts: packedOutputTokenData,
            compressOrDecompressAmount: null,
            isCompress: false,
            cpiContext: null,
            lamportsChangeAccountMerkleTreeIndex: null,
        };
        const data = encodeTransferInstructionData(rawData);

        const {
            accountCompressionAuthority,
            noopProgram,
            registeredProgramPda,
            accountCompressionProgram,
        } = defaultStaticAccountsStruct();
        const keys = transferAccountsLayout({
            feePayer: payer,
            authority,
            cpiAuthorityPda: this.deriveCpiAuthorityPda,
            lightSystemProgram: LightSystemProgram.programId,
            registeredProgramPda: registeredProgramPda,
            noopProgram: noopProgram,
            accountCompressionAuthority: accountCompressionAuthority,
            accountCompressionProgram: accountCompressionProgram,
            selfProgram: this.programId,
            tokenPoolPda: undefined,
            compressOrDecompressTokenAccount: undefined,
            tokenProgram: undefined,
            systemProgram: SystemProgram.programId,
        });

        keys.push(...remainingAccountMetas);

        return new TransactionInstruction({
            programId: this.programId,
            keys,
            data,
        });
    }

    /**
     * Create lookup table instructions for the token program's default
     * accounts.
     *
     * @param payer                     Fee payer.
     * @param authority                 Authority.
     * @param mints                     Mints.
     * @param recentSlot                Recent slot.
     * @param remainingAccounts         Remaining accounts.
     *
     * @returns [createInstruction, extendInstruction, option(extendInstruction2)]
     */
    static async createTokenProgramLookupTable({
        payer,
        authority,
        mints,
        recentSlot,
        remainingAccounts,
    }: CreateTokenProgramLookupTableParams) {
        const [createInstruction, lookupTableAddress] =
            AddressLookupTableProgram.createLookupTable({
                authority,
                payer: authority,
                recentSlot,
            });

        let optionalMintKeys: PublicKey[] = [];
        if (mints) {
            optionalMintKeys = [
                ...mints,
                ...mints.map(mint => this.deriveTokenPoolPda(mint)),
            ];
        }

        const extendInstruction = AddressLookupTableProgram.extendLookupTable({
            payer,
            authority,
            lookupTable: lookupTableAddress,
            addresses: [
                SystemProgram.programId,
                ComputeBudgetProgram.programId,
                this.deriveCpiAuthorityPda,
                LightSystemProgram.programId,
                CompressedTokenProgram.programId,
                defaultStaticAccountsStruct().registeredProgramPda,
                defaultStaticAccountsStruct().noopProgram,
                defaultStaticAccountsStruct().accountCompressionAuthority,
                defaultStaticAccountsStruct().accountCompressionProgram,
                defaultTestStateTreeAccounts().merkleTree,
                defaultTestStateTreeAccounts().nullifierQueue,
                defaultTestStateTreeAccounts().addressTree,
                defaultTestStateTreeAccounts().addressQueue,
                this.programId,
                TOKEN_PROGRAM_ID,
                TOKEN_2022_PROGRAM_ID,
                authority,
                ...optionalMintKeys,
            ],
        });

        const instructions = [createInstruction, extendInstruction];

        if (remainingAccounts && remainingAccounts.length > 0) {
            for (let i = 0; i < remainingAccounts.length; i += 25) {
                const chunk = remainingAccounts.slice(i, i + 25);
                const extendIx = AddressLookupTableProgram.extendLookupTable({
                    payer,
                    authority,
                    lookupTable: lookupTableAddress,
                    addresses: chunk,
                });
                instructions.push(extendIx);
            }
        }

        return {
            instructions,
            address: lookupTableAddress,
        };
    }

    /**
     * Create compress instruction
     *
     * @param payer                         Fee payer.
     * @param owner                         Owner of uncompressed token account.
     * @param source                        Source SPL Token account address.
     * @param toAddress                     Recipient owner address(es).
     * @param amount                        Amount of tokens to compress.
     * @param mint                          SPL Token mint address.
     * @param outputStateTreeInfo           State tree to write to.
     * @param tokenPoolInfo                 Token pool info.
     *
     * @returns The compress instruction
     */
    static async compress({
        payer,
        owner,
        source,
        toAddress,
        amount,
        mint,
        outputStateTreeInfo,
        tokenPoolInfo,
    }: CompressParams): Promise<TransactionInstruction> {
        let tokenTransferOutputs: TokenTransferOutputData[];

        const amountArray = toArray<BN | number>(amount);
        const toAddressArray = toArray(toAddress);

        checkTokenPoolInfo(tokenPoolInfo, mint);

        if (amountArray.length !== toAddressArray.length) {
            throw new Error(
                'Amount and toAddress arrays must have the same length',
            );
        }
        if (featureFlags.isV2()) {
            const [index, bump] = this.findTokenPoolIndexAndBump(
                tokenPoolInfo.tokenPoolPda,
                mint,
            );
            const rawData: BatchCompressInstructionData = {
                pubkeys: toAddressArray,
                amounts:
                    amountArray.length > 1
                        ? amountArray.map(amt => bn(amt))
                        : null,
                lamports: null,
                amount: amountArray.length === 1 ? bn(amountArray[0]) : null,
                index,
                bump,
            };

            const data = encodeBatchCompressInstructionData(rawData);
            const keys = mintToAccountsLayout({
                mint,
                feePayer: payer,
                authority: owner,
                cpiAuthorityPda: this.deriveCpiAuthorityPda,
                tokenProgram: tokenPoolInfo.tokenProgram,
                tokenPoolPda: tokenPoolInfo.tokenPoolPda,
                lightSystemProgram: LightSystemProgram.programId,
                ...defaultStaticAccountsStruct(),
                merkleTree: outputStateTreeInfo.queue,
                selfProgram: this.programId,
                systemProgram: SystemProgram.programId,
                solPoolPda: null,
            });
            keys.push({
                pubkey: source,
                isWritable: true,
                isSigner: false,
            });

            return new TransactionInstruction({
                programId: this.programId,
                keys,
                data,
            });
        } else {
            tokenTransferOutputs = amountArray.map((amt, index) => {
                const amountBN = bn(amt);
                return {
                    owner: toAddressArray[index],
                    amount: amountBN,
                    lamports: null,
                    tlv: null,
                };
            });

            const {
                inputTokenDataWithContext,
                packedOutputTokenData,
                remainingAccountMetas,
            } = packCompressedTokenAccounts({
                inputCompressedTokenAccounts: [],
                outputStateTreeInfo,
                rootIndices: [],
                tokenTransferOutputs,
            });

            const rawData: CompressedTokenInstructionDataTransfer = {
                proof: null,
                mint,
                delegatedTransfer: null,
                inputTokenDataWithContext,
                outputCompressedAccounts: packedOutputTokenData,
                compressOrDecompressAmount: Array.isArray(amount)
                    ? amount
                          .map(amt => bn(amt))
                          .reduce((sum, amt) => sum.add(amt), bn(0))
                    : bn(amount),
                isCompress: true,
                cpiContext: null,
                lamportsChangeAccountMerkleTreeIndex: null,
            };
            const data = encodeTransferInstructionData(rawData);
            const keys = transferAccountsLayout({
                ...defaultStaticAccountsStruct(),
                feePayer: payer,
                authority: owner,
                cpiAuthorityPda: this.deriveCpiAuthorityPda,
                lightSystemProgram: LightSystemProgram.programId,
                selfProgram: this.programId,
                systemProgram: SystemProgram.programId,
                tokenPoolPda: tokenPoolInfo.tokenPoolPda,
                compressOrDecompressTokenAccount: source,
                tokenProgram: tokenPoolInfo.tokenProgram,
            });
            keys.push(...remainingAccountMetas);

            return new TransactionInstruction({
                programId: this.programId,
                keys,
                data,
            });
        }
    }

    /**
     * Construct decompress instruction
     *
     * @param payer                         Fee payer.
     * @param inputCompressedTokenAccounts  Source compressed token accounts.
     * @param toAddress                     Destination **uncompressed** token
     *                                      account address. (ATA)
     * @param amount                        Amount of tokens to decompress.
     * @param recentValidityProof           Recent validity proof.
     * @param recentInputStateRootIndices   Recent state root indices.
     * @param tokenPoolInfos                Token pool info.
     *
     * @returns The decompress instruction
     */
    static async decompress({
        payer,
        inputCompressedTokenAccounts,
        toAddress,
        amount,
        recentValidityProof,
        recentInputStateRootIndices,
        tokenPoolInfos,
    }: DecompressParams): Promise<TransactionInstruction> {
        const amountBN = bn(amount);
        const tokenPoolInfosArray = toArray(tokenPoolInfos);

        const tokenTransferOutputs = createDecompressOutputState(
            inputCompressedTokenAccounts,
            amountBN,
        );

        /// Pack
        const {
            inputTokenDataWithContext,
            packedOutputTokenData,
            remainingAccountMetas,
        } = packCompressedTokenAccounts({
            inputCompressedTokenAccounts,
            rootIndices: recentInputStateRootIndices,
            tokenTransferOutputs: tokenTransferOutputs,
            remainingAccounts: tokenPoolInfosArray
                .slice(1)
                .map(info => info.tokenPoolPda),
        });

        const { mint } = parseTokenData(inputCompressedTokenAccounts);
        const { delegatedTransfer, authority } = parseMaybeDelegatedTransfer(
            inputCompressedTokenAccounts,
            tokenTransferOutputs,
        );

        const rawData: CompressedTokenInstructionDataTransfer = {
            proof: recentValidityProof,
            mint,
            delegatedTransfer,
            inputTokenDataWithContext,
            outputCompressedAccounts: packedOutputTokenData,
            compressOrDecompressAmount: amountBN,
            isCompress: false,
            cpiContext: null,
            lamportsChangeAccountMerkleTreeIndex: null,
        };
        const data = encodeTransferInstructionData(rawData);
        const tokenProgram = tokenPoolInfosArray[0].tokenProgram;

        const {
            accountCompressionAuthority,
            noopProgram,
            registeredProgramPda,
            accountCompressionProgram,
        } = defaultStaticAccountsStruct();

        const keys = transferAccountsLayout({
            feePayer: payer,
            authority: authority,
            cpiAuthorityPda: this.deriveCpiAuthorityPda,
            lightSystemProgram: LightSystemProgram.programId,
            registeredProgramPda: registeredProgramPda,
            noopProgram: noopProgram,
            accountCompressionAuthority: accountCompressionAuthority,
            accountCompressionProgram: accountCompressionProgram,
            selfProgram: this.programId,
            tokenPoolPda: tokenPoolInfosArray[0].tokenPoolPda,
            compressOrDecompressTokenAccount: toAddress,
            tokenProgram,
            systemProgram: SystemProgram.programId,
        });
        keys.push(...remainingAccountMetas);

        return new TransactionInstruction({
            programId: this.programId,
            keys,
            data,
        });
    }

    /**
     * Create `mergeTokenAccounts` instruction
     *
     * @param payer                         Fee payer.
     * @param owner                         Owner of the compressed token
     *                                      accounts to be merged.
     * @param inputCompressedTokenAccounts  Source compressed token accounts.
     * @param mint                          SPL Token mint address.
     * @param recentValidityProof           Recent validity proof.
     * @param recentInputStateRootIndices   Recent state root indices.
     *
     * @returns instruction
     */
    static async mergeTokenAccounts({
        payer,
        owner,
        inputCompressedTokenAccounts,
        mint,
        recentValidityProof,
        recentInputStateRootIndices,
    }: MergeTokenAccountsParams): Promise<TransactionInstruction[]> {
        if (inputCompressedTokenAccounts.length > 4) {
            throw new Error('Cannot merge more than 4 token accounts at once');
        }

        checkMint(inputCompressedTokenAccounts, mint);

        const ix = await this.transfer({
            payer,
            inputCompressedTokenAccounts,
            toAddress: owner,
            amount: inputCompressedTokenAccounts.reduce(
                (sum, account) => sum.add(account.parsed.amount),
                bn(0),
            ),
            recentInputStateRootIndices,
            recentValidityProof,
        });

        return [ix];
    }

    /**
     * Create `compressSplTokenAccount` instruction
     *
     * @param feePayer              Fee payer.
     * @param authority             SPL Token account owner.
     * @param tokenAccount          SPL Token account to compress.
     * @param mint                  SPL Token mint address.
     * @param remainingAmount       Optional: Amount to leave in token account.
     * @param outputStateTreeInfo   State tree to write to.
     * @param tokenPoolInfo         Token pool info.
     *
     * @returns instruction
     */
    static async compressSplTokenAccount({
        feePayer,
        authority,
        tokenAccount,
        mint,
        remainingAmount,
        outputStateTreeInfo,
        tokenPoolInfo,
    }: CompressSplTokenAccountParams): Promise<TransactionInstruction> {
        checkTokenPoolInfo(tokenPoolInfo, mint);
        const remainingAccountMetas: AccountMeta[] = [
            {
                pubkey:
                    outputStateTreeInfo.treeType === TreeType.StateV2
                        ? outputStateTreeInfo.queue
                        : outputStateTreeInfo.tree,
                isSigner: false,
                isWritable: true,
            },
        ];

        const data = encodeCompressSplTokenAccountInstructionData({
            owner: authority,
            remainingAmount: remainingAmount ?? null,
            cpiContext: null,
        });
        const {
            accountCompressionAuthority,
            noopProgram,
            registeredProgramPda,
            accountCompressionProgram,
        } = defaultStaticAccountsStruct();

        const keys = transferAccountsLayout({
            feePayer,
            authority,
            cpiAuthorityPda: this.deriveCpiAuthorityPda,
            lightSystemProgram: LightSystemProgram.programId,
            registeredProgramPda: registeredProgramPda,
            noopProgram: noopProgram,
            accountCompressionAuthority: accountCompressionAuthority,
            accountCompressionProgram: accountCompressionProgram,
            selfProgram: this.programId,
            tokenPoolPda: tokenPoolInfo.tokenPoolPda,
            compressOrDecompressTokenAccount: tokenAccount,
            tokenProgram: tokenPoolInfo.tokenProgram,
            systemProgram: SystemProgram.programId,
        });

        keys.push(...remainingAccountMetas);

        return new TransactionInstruction({
            programId: this.programId,
            keys,
            data,
        });
    }

    /**
     * Get the program ID for a mint
     *
     * @param mint                  SPL Token mint address.
     * @param connection            Connection.
     *
     * @returns program ID
     */
    static async getMintProgramId(
        mint: PublicKey,
        connection: Connection,
    ): Promise<PublicKey | undefined> {
        return (await connection.getAccountInfo(mint))?.owner;
    }

    /**
     * Create `approve` instruction to delegate compressed tokens.
     *
     * @param payer                         Fee payer.
     * @param inputCompressedTokenAccounts  Source compressed token accounts.
     * @param toAddress                     Owner to delegate to.
     * @param amount                        Amount of tokens to delegate.
     * @param recentValidityProof           Recent validity proof.
     * @param recentInputStateRootIndices   Recent state root indices.
     *
     * @returns instruction
     */
    static async approve({
        payer,
        inputCompressedTokenAccounts,
        toAddress,
        amount,
        recentValidityProof,
        recentInputStateRootIndices,
    }: ApproveParams): Promise<TransactionInstruction> {
        const { inputTokenDataWithContext, remainingAccountMetas } =
            packCompressedTokenAccounts({
                inputCompressedTokenAccounts,
                rootIndices: recentInputStateRootIndices,
                tokenTransferOutputs: [],
            });

        const { mint, currentOwner } = parseTokenData(
            inputCompressedTokenAccounts,
        );

        const CHANGE_INDEX =
            inputCompressedTokenAccounts[0].compressedAccount.treeInfo
                .treeType === TreeType.StateV2
                ? 1
                : 0;

        const rawData: CompressedTokenInstructionDataApprove = {
            proof: recentValidityProof,
            mint,
            inputTokenDataWithContext,
            cpiContext: null,
            delegate: toAddress,
            delegatedAmount: bn(amount),
            delegateMerkleTreeIndex: CHANGE_INDEX,
            changeAccountMerkleTreeIndex: CHANGE_INDEX,
            delegateLamports: null,
        };

        const data = encodeApproveInstructionData(rawData);

        const {
            accountCompressionAuthority,
            noopProgram,
            registeredProgramPda,
            accountCompressionProgram,
        } = defaultStaticAccountsStruct();

        const keys = approveAccountsLayout({
            feePayer: payer,
            authority: currentOwner,
            cpiAuthorityPda: this.deriveCpiAuthorityPda,
            lightSystemProgram: LightSystemProgram.programId,
            registeredProgramPda: registeredProgramPda,
            noopProgram: noopProgram,
            accountCompressionAuthority: accountCompressionAuthority,
            accountCompressionProgram: accountCompressionProgram,
            selfProgram: this.programId,
            systemProgram: SystemProgram.programId,
        });

        keys.push(...remainingAccountMetas);

        return new TransactionInstruction({
            programId: this.programId,
            keys,
            data,
        });
    }

    /**
     * Create `revoke` instruction to revoke delegation of compressed tokens.
     *
     * @param payer                         Fee payer.
     * @param inputCompressedTokenAccounts  Source compressed token accounts.
     * @param recentValidityProof           Recent validity proof.
     * @param recentInputStateRootIndices   Recent state root indices.
     *
     * @returns instruction
     */
    static async revoke({
        payer,
        inputCompressedTokenAccounts,
        recentValidityProof,
        recentInputStateRootIndices,
    }: RevokeParams): Promise<TransactionInstruction> {
        validateSameTokenOwner(inputCompressedTokenAccounts);

        const { inputTokenDataWithContext, remainingAccountMetas } =
            packCompressedTokenAccounts({
                inputCompressedTokenAccounts,
                rootIndices: recentInputStateRootIndices,
                tokenTransferOutputs: [],
            });

        const { mint, currentOwner } = parseTokenData(
            inputCompressedTokenAccounts,
        );

        const rawData: CompressedTokenInstructionDataRevoke = {
            proof: recentValidityProof,
            mint,
            inputTokenDataWithContext,
            cpiContext: null,
            outputAccountMerkleTreeIndex:
                inputCompressedTokenAccounts[0].compressedAccount.treeInfo
                    .treeType === TreeType.StateV2
                    ? 2
                    : 1,
        };
        const data = encodeRevokeInstructionData(rawData);

        const {
            accountCompressionAuthority,
            noopProgram,
            registeredProgramPda,
            accountCompressionProgram,
        } = defaultStaticAccountsStruct();
        const keys = revokeAccountsLayout({
            feePayer: payer,
            authority: currentOwner,
            cpiAuthorityPda: this.deriveCpiAuthorityPda,
            lightSystemProgram: LightSystemProgram.programId,
            registeredProgramPda: registeredProgramPda,
            noopProgram: noopProgram,
            accountCompressionAuthority: accountCompressionAuthority,
            accountCompressionProgram: accountCompressionProgram,
            selfProgram: this.programId,
            systemProgram: SystemProgram.programId,
        });

        keys.push(...remainingAccountMetas);

        return new TransactionInstruction({
            programId: this.programId,
            keys,
            data,
        });
    }
}
