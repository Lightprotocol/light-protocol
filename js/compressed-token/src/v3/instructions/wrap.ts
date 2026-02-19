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
    createCompressSpl,
    createDecompressCtoken,
    Transfer2InstructionData,
    Compression,
} from '../layout/layout-transfer2';

/**
 * Create a wrap instruction that moves tokens from an SPL/T22 account to a
 * light-token account.
 *
 * @param source            Source SPL/T22 token account
 * @param destination       Destination light-token account
 * @param owner             Owner of the source account (signer)
 * @param mint              Mint address
 * @param amount            Amount to wrap,
 * @param splInterfaceInfo  SPL interface info for the compression
 * @param decimals          Mint decimals (required for transfer_checked)
 * @param payer             Fee payer (defaults to owner)
 * @param maxTopUp          Optional cap on rent top-up (units of 1k lamports; default no cap)
 * @returns Instruction to wrap tokens
 */
export function createWrapInstruction(
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

    const compressions: Compression[] = [
        createCompressSpl(
            amount,
            MINT_INDEX,
            SOURCE_INDEX,
            OWNER_INDEX,
            POOL_INDEX,
            splInterfaceInfo.poolIndex,
            splInterfaceInfo.bump,
            decimals,
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
        // System program needed for top-up CPIs when destination has compressible extension
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
