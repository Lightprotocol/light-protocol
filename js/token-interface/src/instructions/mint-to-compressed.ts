import {
    SystemProgram,
    TransactionInstruction,
} from '@solana/web3.js';
import { Buffer } from 'buffer';
import {
    LIGHT_TOKEN_PROGRAM_ID,
    LightSystemProgram,
    defaultStaticAccountsStruct,
    getOutputQueue,
} from '@lightprotocol/stateless.js';
import {
    MAX_TOP_UP,
    TokenDataVersion,
    deriveCpiAuthorityPda,
} from '../constants';
import {
    MintActionCompressedInstructionData,
    encodeMintActionInstructionData,
} from './layout/layout-mint-action';
import type { CreateRawMintToCompressedInstructionInput } from '../types';

/**
 * Create instruction for minting from a light mint directly to compressed token accounts.
 */
export function createMintToCompressedInstruction({
    authority,
    payer,
    validityProof,
    merkleContext,
    mintData,
    recipients,
    outputStateTreeInfo,
    tokenAccountVersion = TokenDataVersion.ShaFlat,
    maxTopUp,
}: CreateRawMintToCompressedInstructionInput): TransactionInstruction {
    if (mintData.metadata) {
        throw new Error(
            'TokenMetadata extension not supported in mintToCompressed instruction',
        );
    }
    if (validityProof.rootIndices.length === 0) {
        throw new Error('Missing root index for mintToCompressed instruction.');
    }

    const isDecompressed = mintData.mintDecompressed;
    const mintSigner = Array.from(mintData.mintSigner);
    const instructionData: MintActionCompressedInstructionData = {
        leafIndex: merkleContext.leafIndex,
        proveByIndex: true,
        rootIndex: validityProof.rootIndices[0],
        maxTopUp: maxTopUp ?? MAX_TOP_UP,
        createMint: null,
        actions: [
            {
                mintToCompressed: {
                    tokenAccountVersion,
                    recipients: recipients.map(recipient => ({
                        recipient: recipient.recipient,
                        amount: BigInt(recipient.amount.toString()),
                    })),
                },
            },
        ],
        proof: isDecompressed ? null : validityProof.compressedProof,
        cpiContext: null,
        mint: isDecompressed
            ? null
            : {
                  supply: mintData.supply,
                  decimals: mintData.decimals,
                  metadata: {
                      version: mintData.version,
                      cmintDecompressed: mintData.mintDecompressed,
                      mint: mintData.splMint,
                      mintSigner,
                      bump: mintData.bump,
                  },
                  mintAuthority: mintData.mintAuthority,
                  freezeAuthority: mintData.freezeAuthority,
                  extensions: null,
              },
    };

    const outputQueue =
        outputStateTreeInfo?.queue ?? getOutputQueue(merkleContext);
    const sys = defaultStaticAccountsStruct();

    return new TransactionInstruction({
        programId: LIGHT_TOKEN_PROGRAM_ID,
        keys: [
            {
                pubkey: LightSystemProgram.programId,
                isSigner: false,
                isWritable: false,
            },
            { pubkey: authority, isSigner: true, isWritable: false },
            ...(isDecompressed
                ? [
                      {
                          pubkey: mintData.splMint,
                          isSigner: false,
                          isWritable: true,
                      },
                  ]
                : []),
            { pubkey: payer, isSigner: true, isWritable: true },
            {
                pubkey: deriveCpiAuthorityPda(),
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: sys.registeredProgramPda,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: sys.accountCompressionAuthority,
                isSigner: false,
                isWritable: false,
            },
            {
                pubkey: sys.accountCompressionProgram,
                isSigner: false,
                isWritable: false,
            },
            { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
            { pubkey: outputQueue, isSigner: false, isWritable: true },
            { pubkey: merkleContext.treeInfo.tree, isSigner: false, isWritable: true },
            {
                pubkey: merkleContext.treeInfo.queue,
                isSigner: false,
                isWritable: true,
            },
            { pubkey: outputQueue, isSigner: false, isWritable: true },
        ],
        data: encodeMintActionInstructionData(instructionData),
    });
}
