import {
    PublicKey,
    TransactionInstruction,
    SystemProgram,
} from '@solana/web3.js';
import {
    CTOKEN_PROGRAM_ID,
    LightSystemProgram,
    defaultStaticAccountsStruct,
    ParsedTokenAccount,
    bn,
    CompressedProof,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../../program';
import {
    encodeTransfer2InstructionData,
    Transfer2InstructionData,
    MultiInputTokenDataWithContext,
    COMPRESSION_MODE_DECOMPRESS,
    Compression,
} from '../../layout-transfer2';
import { TokenDataVersion } from '../../constants';

/**
 * Build input token data for Transfer2 from parsed token accounts
 */
function buildInputTokenData(
    accounts: ParsedTokenAccount[],
    rootIndices: number[],
    packedAccountIndices: Map<string, number>,
): MultiInputTokenDataWithContext[] {
    return accounts.map((acc, i) => {
        const ownerKey = acc.compressedAccount.owner.toBase58();
        const mintKey = acc.parsed.mint.toBase58();

        return {
            owner: packedAccountIndices.get(ownerKey)!,
            amount: BigInt(acc.parsed.amount.toString()),
            hasDelegate: acc.parsed.delegate !== null,
            delegate: acc.parsed.delegate
                ? (packedAccountIndices.get(acc.parsed.delegate.toBase58()) ??
                  0)
                : 0,
            mint: packedAccountIndices.get(mintKey)!,
            version: TokenDataVersion.ShaFlat,
            merkleContext: {
                merkleTreePubkeyIndex: packedAccountIndices.get(
                    acc.compressedAccount.treeInfo.tree.toBase58(),
                )!,
                queuePubkeyIndex: packedAccountIndices.get(
                    acc.compressedAccount.treeInfo.queue.toBase58(),
                )!,
                leafIndex: acc.compressedAccount.leafIndex,
                proveByIndex: acc.compressedAccount.proveByIndex,
            },
            rootIndex: rootIndices[i],
        };
    });
}

/**
 * Create decompress2 instruction using Transfer2.
 *
 * This decompresses compressed tokens to a CToken account using the unified
 * Transfer2 instruction. It's more efficient than the old decompress as it
 * doesn't require SPL token pool operations for CToken destinations.
 *
 * @param payer                        Fee payer public key
 * @param inputCompressedTokenAccounts Input compressed token accounts
 * @param toAddress                    Destination CToken account address
 * @param amount                       Amount to decompress
 * @param proof                        Validity proof (null if all accounts are proveByIndex)
 * @param rootIndices                  Root indices for each input account
 * @returns TransactionInstruction
 */
export function createDecompress2Instruction(
    payer: PublicKey,
    inputCompressedTokenAccounts: ParsedTokenAccount[],
    toAddress: PublicKey,
    amount: bigint,
    proof: CompressedProof | null,
    rootIndices: number[],
): TransactionInstruction {
    if (inputCompressedTokenAccounts.length === 0) {
        throw new Error('No input compressed token accounts provided');
    }

    const mint = inputCompressedTokenAccounts[0].parsed.mint;
    const owner = inputCompressedTokenAccounts[0].compressedAccount.owner;

    // Build packed accounts map
    // Order: trees/queues first, then mint, owner, CToken account, CToken program
    const packedAccountIndices = new Map<string, number>();
    const packedAccounts: PublicKey[] = [];

    // Collect unique trees and queues
    const treeSet = new Set<string>();
    const queueSet = new Set<string>();
    for (const acc of inputCompressedTokenAccounts) {
        treeSet.add(acc.compressedAccount.treeInfo.tree.toBase58());
        queueSet.add(acc.compressedAccount.treeInfo.queue.toBase58());
    }

    // Add trees first (owned by account compression program)
    for (const tree of treeSet) {
        packedAccountIndices.set(tree, packedAccounts.length);
        packedAccounts.push(new PublicKey(tree));
    }

    // Add queues
    for (const queue of queueSet) {
        packedAccountIndices.set(queue, packedAccounts.length);
        packedAccounts.push(new PublicKey(queue));
    }

    // Add mint
    const mintIndex = packedAccounts.length;
    packedAccountIndices.set(mint.toBase58(), mintIndex);
    packedAccounts.push(mint);

    // Add owner
    const ownerIndex = packedAccounts.length;
    packedAccountIndices.set(owner.toBase58(), ownerIndex);
    packedAccounts.push(owner);

    // Add destination CToken account
    const destinationIndex = packedAccounts.length;
    packedAccountIndices.set(toAddress.toBase58(), destinationIndex);
    packedAccounts.push(toAddress);

    // Add CToken program (for decompress to CToken)
    const ctokenProgramIndex = packedAccounts.length;
    packedAccounts.push(CTOKEN_PROGRAM_ID);

    // Build input token data
    const inTokenData = buildInputTokenData(
        inputCompressedTokenAccounts,
        rootIndices,
        packedAccountIndices,
    );

    // Build decompress compression
    const compressions: Compression[] = [
        {
            mode: COMPRESSION_MODE_DECOMPRESS,
            amount,
            mint: mintIndex,
            sourceOrRecipient: destinationIndex,
            authority: 0, // Not needed for decompress
            poolAccountIndex: ctokenProgramIndex, // CToken program
            poolIndex: 0,
            bump: 0,
        },
    ];

    // Build Transfer2 instruction data
    const instructionData: Transfer2InstructionData = {
        withTransactionHash: false,
        withLamportsChangeAccountMerkleTreeIndex: false,
        lamportsChangeAccountMerkleTreeIndex: 0,
        lamportsChangeAccountOwnerIndex: 0,
        outputQueue: 0, // First queue in packed accounts
        cpiContext: null,
        compressions,
        proof: proof
            ? {
                  a: Array.from(proof.a),
                  b: Array.from(proof.b),
                  c: Array.from(proof.c),
              }
            : null,
        inTokenData,
        outTokenData: [], // No compressed outputs
        inLamports: null,
        outLamports: null,
        inTlv: null,
        outTlv: null,
    };

    const data = encodeTransfer2InstructionData(instructionData);

    // Build accounts for Transfer2 with compressed accounts (full path)
    const {
        accountCompressionAuthority,
        noopProgram,
        registeredProgramPda,
        accountCompressionProgram,
    } = defaultStaticAccountsStruct();

    const keys = [
        // 0: light_system_program (non-mutable)
        {
            pubkey: LightSystemProgram.programId,
            isSigner: false,
            isWritable: false,
        },
        // 1: fee_payer (signer, mutable)
        { pubkey: payer, isSigner: true, isWritable: true },
        // 2: authority (signer)
        { pubkey: owner, isSigner: true, isWritable: false },
        // 3: cpi_authority_pda
        {
            pubkey: CompressedTokenProgram.deriveCpiAuthorityPda,
            isSigner: false,
            isWritable: false,
        },
        // 4: registered_program_pda
        {
            pubkey: registeredProgramPda,
            isSigner: false,
            isWritable: false,
        },
        // 5: account_compression_authority
        {
            pubkey: accountCompressionAuthority,
            isSigner: false,
            isWritable: false,
        },
        // 6: account_compression_program
        {
            pubkey: accountCompressionProgram,
            isSigner: false,
            isWritable: false,
        },
        // 7: system_program
        {
            pubkey: SystemProgram.programId,
            isSigner: false,
            isWritable: false,
        },
        // 8: noop_program (for logging)
        {
            pubkey: noopProgram,
            isSigner: false,
            isWritable: false,
        },
        // Packed accounts (trees/queues come first, identified by ownership)
        ...packedAccounts.map((pubkey, i) => {
            // Trees and destination CToken account need to be writable
            const isTreeOrQueue = i < treeSet.size + queueSet.size;
            const isDestination = pubkey.equals(toAddress);
            return {
                pubkey,
                isSigner: false,
                isWritable: isTreeOrQueue || isDestination,
            };
        }),
    ];

    return new TransactionInstruction({
        programId: CompressedTokenProgram.programId,
        keys,
        data,
    });
}
