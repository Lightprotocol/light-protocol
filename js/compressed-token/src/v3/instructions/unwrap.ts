import {
    PublicKey,
    TransactionInstruction,
    SystemProgram,
} from '@solana/web3.js';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import { MAX_TOP_UP } from '../../constants';
import { CompressedTokenProgram } from '../../program';
import { SplInterfaceInfo } from '../../utils/get-token-pool-infos';
import {
    encodeTransfer2InstructionData,
    createCompressCtoken,
    createDecompressSpl,
    Transfer2InstructionData,
    Compression,
} from '../layout/layout-transfer2';

/**
 * Create an unwrap instruction that moves tokens from a c-token account to an
 * SPL/T22 account.
 *
 * @param source           Source c-token account
 * @param destination      Destination SPL/T22 token account
 * @param owner            Owner of the source account (signer)
 * @param mint             Mint address
 * @param amount           Amount to unwrap,
 * @param splInterfaceInfo SPL interface info for the decompression
 * @param decimals         Mint decimals (required for transfer_checked)
 * @param payer            Fee payer (defaults to owner if not provided)
 * @param maxTopUp         Optional cap on rent top-up (units of 1k lamports; default no cap)
 * @returns TransactionInstruction to unwrap tokens
 */
export function createUnwrapInstruction(
    source: PublicKey,
    destination: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    amount: bigint,
    splInterfaceInfo: SplInterfaceInfo,
    decimals: number,
    payer: PublicKey = owner,
    maxTopUp?: number,
): TransactionInstruction {
    const MINT_INDEX = 0;
    const OWNER_INDEX = 1;
    const SOURCE_INDEX = 2;
    const DESTINATION_INDEX = 3;
    const POOL_INDEX = 4;
    const _SPL_TOKEN_PROGRAM_INDEX = 5;
    const CTOKEN_PROGRAM_INDEX = 6;

    // Unwrap flow: compress from c-token, decompress to SPL
    const compressions: Compression[] = [
        createCompressCtoken(
            amount,
            MINT_INDEX,
            SOURCE_INDEX,
            OWNER_INDEX,
            CTOKEN_PROGRAM_INDEX,
        ),
        createDecompressSpl(
            amount,
            MINT_INDEX,
            DESTINATION_INDEX,
            POOL_INDEX,
            splInterfaceInfo.poolIndex,
            splInterfaceInfo.bump,
            decimals,
        ),
    ];

    const instructionData: Transfer2InstructionData = {
        withTransactionHash: false,
        withLamportsChangeAccountMerkleTreeIndex: false,
        lamportsChangeAccountMerkleTreeIndex: 0,
        lamportsChangeAccountOwnerIndex: 0,
        outputQueue: 0,
        maxTopUp: maxTopUp ?? MAX_TOP_UP,
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

    // Account order matches wrap instruction for consistency
    const keys = [
        {
            pubkey: CompressedTokenProgram.deriveCpiAuthorityPda,
            isSigner: false,
            isWritable: false,
        },
        { pubkey: payer, isSigner: true, isWritable: true },
        { pubkey: mint, isSigner: false, isWritable: false },
        { pubkey: owner, isSigner: true, isWritable: false },
        { pubkey: source, isSigner: false, isWritable: true },
        { pubkey: destination, isSigner: false, isWritable: true },
        {
            pubkey: splInterfaceInfo.splInterfacePda,
            isSigner: false,
            isWritable: true,
        },
        {
            pubkey: splInterfaceInfo.tokenProgram,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: LIGHT_TOKEN_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
        },
        // System program needed for top-up CPIs when source has compressible extension
        {
            pubkey: SystemProgram.programId,
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
