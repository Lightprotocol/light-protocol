import {
    PublicKey,
    SystemProgram,
    TransactionInstruction,
} from '@solana/web3.js';
import { Buffer } from 'buffer';
import {
    ValidityProofWithContext,
    CTOKEN_PROGRAM_ID,
    LightSystemProgram,
    defaultStaticAccountsStruct,
    getOutputQueue,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../../program';
import { MintInterface } from '../get-mint-interface';
import {
    encodeMintActionInstructionData,
    MintActionCompressedInstructionData,
    ExtensionInstructionData,
} from '../layout/layout-mint-action';
import { LIGHT_TOKEN_CONFIG, LIGHT_TOKEN_RENT_SPONSOR } from '../../constants';

interface EncodeDecompressMintInstructionParams {
    leafIndex: number;
    proveByIndex: boolean;
    rootIndex: number;
    proof: { a: number[]; b: number[]; c: number[] } | null;
    mintInterface: MintInterface;
    rentPayment: number;
    writeTopUp: number;
}

function encodeDecompressMintInstructionData(
    params: EncodeDecompressMintInstructionParams,
): Buffer {
    // Build extensions if metadata present
    let extensions: ExtensionInstructionData[] | null = null;
    if (params.mintInterface.tokenMetadata) {
        extensions = [
            {
                tokenMetadata: {
                    updateAuthority:
                        params.mintInterface.tokenMetadata.updateAuthority ??
                        null,
                    name: Buffer.from(params.mintInterface.tokenMetadata.name),
                    symbol: Buffer.from(
                        params.mintInterface.tokenMetadata.symbol,
                    ),
                    uri: Buffer.from(params.mintInterface.tokenMetadata.uri),
                    additionalMetadata: null,
                },
            },
        ];
    }

    const instructionData: MintActionCompressedInstructionData = {
        leafIndex: params.leafIndex,
        proveByIndex: params.proveByIndex,
        rootIndex: params.rootIndex,
        maxTopUp: 0,
        createMint: null,
        actions: [
            {
                decompressMint: {
                    rentPayment: params.rentPayment,
                    writeTopUp: params.writeTopUp,
                },
            },
        ],
        proof: params.proof,
        cpiContext: null,
        mint: {
            supply: params.mintInterface.mint.supply,
            decimals: params.mintInterface.mint.decimals,
            metadata: {
                version: params.mintInterface.mintContext!.version,
                cmintDecompressed:
                    params.mintInterface.mintContext!.cmintDecompressed,
                mint: params.mintInterface.mintContext!.splMint,
                mintSigner: Array.from(
                    params.mintInterface.mintContext!.mintSigner,
                ),
                bump: params.mintInterface.mintContext!.bump,
            },
            mintAuthority: params.mintInterface.mint.mintAuthority,
            freezeAuthority: params.mintInterface.mint.freezeAuthority,
            extensions,
        },
    };

    return encodeMintActionInstructionData(instructionData);
}

export interface DecompressMintInstructionParams {
    /** MintInterface from getMintInterface() - must have merkleContext */
    mintInterface: MintInterface;
    /** Authority signer public key (can be any account, decompressMint is permissionless) */
    authority: PublicKey;
    /** Fee payer public key */
    payer: PublicKey;
    /** Validity proof for the compressed mint */
    validityProof: ValidityProofWithContext;
    /** Number of epochs to prepay rent (minimum 2) */
    rentPayment?: number;
    /** Per-write top-up in lamports (default: 766) */
    writeTopUp?: number;
    /** Compressible config account (default: LIGHT_TOKEN_CONFIG) */
    configAccount?: PublicKey;
    /** Rent sponsor PDA (default: LIGHT_TOKEN_RENT_SPONSOR) */
    rentSponsor?: PublicKey;
}

/**
 * Create instruction for decompressing a compressed mint.
 *
 * This creates the CMint Solana account from a compressed mint, making
 * the mint available on-chain. This is required before creating CToken
 * associated token accounts.
 *
 * DecompressMint is **permissionless** - any account can call it. The
 * caller pays initial rent, rent exemption is sponsored by the rent_sponsor.
 *
 * @param params - Instruction parameters
 * @returns TransactionInstruction for decompressing the mint
 */
export function createDecompressMintInstruction(
    params: DecompressMintInstructionParams,
): TransactionInstruction {
    const {
        mintInterface,
        authority,
        payer,
        validityProof,
        rentPayment = 16, // Default: 16 epochs (~24 hours)
        writeTopUp = 766, // Default: ~2 epochs worth
        configAccount = LIGHT_TOKEN_CONFIG,
        rentSponsor = LIGHT_TOKEN_RENT_SPONSOR,
    } = params;

    if (!mintInterface.merkleContext) {
        throw new Error(
            'MintInterface must have merkleContext for compressed mint operations',
        );
    }
    if (!mintInterface.mintContext) {
        throw new Error(
            'MintInterface must have mintContext for compressed mint operations',
        );
    }

    // Validate rentPayment minimum
    if (rentPayment < 2) {
        throw new Error('rentPayment must be at least 2 epochs');
    }

    const merkleContext = mintInterface.merkleContext;
    const outputQueue = getOutputQueue(merkleContext);

    const data = encodeDecompressMintInstructionData({
        leafIndex: merkleContext.leafIndex,
        proveByIndex: true,
        rootIndex: validityProof.rootIndices[0],
        proof: validityProof.compressedProof,
        mintInterface,
        rentPayment,
        writeTopUp,
    });

    const sys = defaultStaticAccountsStruct();

    // Account order for decompressMint (needs_compressible_accounts = true):
    // 0. light_system_program
    // 1. authority (signer)
    // 2. compressible_config
    // 3. cmint (to be created)
    // 4. rent_sponsor
    // 5. fee_payer (signer, mut)
    // 6. registered_program_pda
    // 7. account_compression_authority
    // 8. account_compression_program
    // 9. system_program
    // 10. out_output_queue
    // 11. in_merkle_tree
    // 12. in_output_queue
    const keys = [
        {
            pubkey: LightSystemProgram.programId,
            isSigner: false,
            isWritable: false,
        },
        { pubkey: authority, isSigner: true, isWritable: false },
        { pubkey: configAccount, isSigner: false, isWritable: false },
        {
            pubkey: mintInterface.mintContext.splMint,
            isSigner: false,
            isWritable: true,
        },
        { pubkey: rentSponsor, isSigner: false, isWritable: true },
        { pubkey: payer, isSigner: true, isWritable: true },
        {
            pubkey: CompressedTokenProgram.deriveCpiAuthorityPda,
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
        {
            pubkey: merkleContext.treeInfo.tree,
            isSigner: false,
            isWritable: true,
        },
        {
            pubkey: merkleContext.treeInfo.queue,
            isSigner: false,
            isWritable: true,
        },
    ];

    return new TransactionInstruction({
        programId: CTOKEN_PROGRAM_ID,
        keys,
        data,
    });
}
