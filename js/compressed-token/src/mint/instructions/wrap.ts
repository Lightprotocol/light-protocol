import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { CTOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../../program';
import { TokenPoolInfo } from '../../utils/get-token-pool-infos';
import {
    encodeTransfer2InstructionData,
    createCompressSpl,
    createDecompressCtoken,
    Transfer2InstructionData,
    Compression,
} from '../../layout-transfer2';

// Keep old interface type for backwards compatibility export
export interface CreateWrapInstructionParams {
    source: PublicKey;
    destination: PublicKey;
    owner: PublicKey;
    mint: PublicKey;
    amount: bigint;
    tokenPoolInfo: TokenPoolInfo;
    payer?: PublicKey;
}

/**
 * Create a wrap instruction that moves tokens from an SPL/T22 account to a CToken account.
 *
 * This is an agnostic, low-level instruction that takes explicit account addresses.
 * Use the wrap() action for a higher-level convenience wrapper.
 *
 * The wrap operation:
 * 1. Compresses tokens from the SPL/T22 source account into the token pool
 * 2. Decompresses tokens from the pool to the CToken destination account
 *
 * @param source        Source SPL/T22 token account (any token account, not just ATA)
 * @param destination   Destination CToken account (any CToken account, not just ATA)
 * @param owner         Owner/authority of the source account (must sign)
 * @param mint          Mint address
 * @param amount        Amount to wrap
 * @param tokenPoolInfo Token pool info for the compression
 * @param payer         Fee payer (defaults to owner if not provided)
 * @returns TransactionInstruction to wrap tokens
 */
export function createWrapInstruction(
    source: PublicKey,
    destination: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    amount: bigint,
    tokenPoolInfo: TokenPoolInfo,
    payer: PublicKey = owner,
): TransactionInstruction {
    // Account indices in packed accounts (after fixed accounts):
    // 0 = mint
    // 1 = owner/authority
    // 2 = source (SPL/T22 token account)
    // 3 = destination (CToken account)
    // 4 = token pool PDA
    // 5 = SPL token program (for compress)
    // 6 = CToken program (for decompress to CToken)
    const MINT_INDEX = 0;
    const OWNER_INDEX = 1;
    const SOURCE_INDEX = 2;
    const DESTINATION_INDEX = 3;
    const POOL_INDEX = 4;
    const SPL_TOKEN_PROGRAM_INDEX = 5;
    const CTOKEN_PROGRAM_INDEX = 6;

    // Build compressions:
    // 1. Compress from source (tokens go to pool)
    // 2. Decompress to destination (CToken balance increases)
    const compressions: Compression[] = [
        createCompressSpl(
            amount,
            MINT_INDEX,
            SOURCE_INDEX,
            OWNER_INDEX,
            POOL_INDEX,
            tokenPoolInfo.poolIndex,
            tokenPoolInfo.bump,
        ),
        createDecompressCtoken(
            amount,
            MINT_INDEX,
            DESTINATION_INDEX,
            CTOKEN_PROGRAM_INDEX,
        ),
    ];

    const instructionData: Transfer2InstructionData = {
        withTransactionHash: false,
        withLamportsChangeAccountMerkleTreeIndex: false,
        lamportsChangeAccountMerkleTreeIndex: 0,
        lamportsChangeAccountOwnerIndex: 0,
        outputQueue: 0,
        cpiContext: null,
        compressions,
        proof: null,
        inTokenData: [],
        outTokenData: [],
        inLamports: null,
        outLamports: null,
        inTlv: null,
        outTlv: null,
    };

    const data = encodeTransfer2InstructionData(instructionData);

    // Accounts for compressions-only path:
    // 0: compressions_only_cpi_authority_pda
    // 1: compressions_only_fee_payer (signer)
    // Then packed accounts: mint, owner, source, destination, pool, spl_program, ctoken_program
    const keys = [
        // Fixed accounts for compressions-only
        {
            pubkey: CompressedTokenProgram.deriveCpiAuthorityPda,
            isSigner: false,
            isWritable: false,
        },
        { pubkey: payer, isSigner: true, isWritable: true },
        // Packed accounts
        { pubkey: mint, isSigner: false, isWritable: false },
        { pubkey: owner, isSigner: true, isWritable: false },
        { pubkey: source, isSigner: false, isWritable: true },
        { pubkey: destination, isSigner: false, isWritable: true },
        {
            pubkey: tokenPoolInfo.tokenPoolPda,
            isSigner: false,
            isWritable: true,
        },
        // SPL token program for compress
        {
            pubkey: tokenPoolInfo.tokenProgram,
            isSigner: false,
            isWritable: false,
        },
        // CToken program for decompress to CToken
        {
            pubkey: CTOKEN_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
        },
    ];

    return new TransactionInstruction({
        programId: CompressedTokenProgram.programId,
        keys,
        data,
    });
}
