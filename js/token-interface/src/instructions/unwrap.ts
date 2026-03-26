import {
    PublicKey,
    TransactionInstruction,
    SystemProgram,
} from '@solana/web3.js';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import {
    COMPRESSED_TOKEN_PROGRAM_ID,
    deriveCpiAuthorityPda,
    MAX_TOP_UP,
} from '../constants';
import type { SplPoolInfo } from '../spl-interface';
import {
    encodeTransfer2InstructionData,
    createCompressLightToken,
    createDecompressSpl,
    type Transfer2InstructionData,
    type Compression,
} from './layout/layout-transfer2';

/**
 * Create an unwrap instruction that moves tokens from a light-token account to an
 * SPL/T22 account.
 */
export function createUnwrapInstruction(
    source: PublicKey,
    destination: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    amount: bigint,
    splPoolInfo: SplPoolInfo,
    decimals: number,
    payer: PublicKey = owner,
    maxTopUp?: number,
): TransactionInstruction {
    const MINT_INDEX = 0;
    const OWNER_INDEX = 1;
    const SOURCE_INDEX = 2;
    const DESTINATION_INDEX = 3;
    const POOL_INDEX = 4;
    const LIGHT_TOKEN_PROGRAM_INDEX = 6;

    const compressions: Compression[] = [
        createCompressLightToken(
            amount,
            MINT_INDEX,
            SOURCE_INDEX,
            OWNER_INDEX,
            LIGHT_TOKEN_PROGRAM_INDEX,
        ),
        createDecompressSpl(
            amount,
            MINT_INDEX,
            DESTINATION_INDEX,
            POOL_INDEX,
            splPoolInfo.poolIndex,
            splPoolInfo.bump,
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

    const keys = [
        {
            pubkey: deriveCpiAuthorityPda(),
            isSigner: false,
            isWritable: false,
        },
        { pubkey: payer, isSigner: true, isWritable: true },
        { pubkey: mint, isSigner: false, isWritable: false },
        { pubkey: owner, isSigner: true, isWritable: false },
        { pubkey: source, isSigner: false, isWritable: true },
        { pubkey: destination, isSigner: false, isWritable: true },
        {
            pubkey: splPoolInfo.splPoolPda,
            isSigner: false,
            isWritable: true,
        },
        {
            pubkey: splPoolInfo.tokenProgram,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: LIGHT_TOKEN_PROGRAM_ID,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: SystemProgram.programId,
            isSigner: false,
            isWritable: false,
        },
    ];

    return new TransactionInstruction({
        programId: COMPRESSED_TOKEN_PROGRAM_ID,
        keys,
        data,
    });
}
