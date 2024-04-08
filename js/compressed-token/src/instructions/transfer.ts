import {
    CompressedProof,
    defaultStaticAccountsStruct,
    CompressedAccountWithMerkleContext,
    LightSystemProgram,
    bn,
} from '@lightprotocol/stateless.js';
import {
    PublicKey,
    TransactionInstruction,
    AccountMeta,
    ComputeBudgetProgram,
} from '@solana/web3.js';
import { CompressedTokenProgram } from '../program';
import {
    CompressedTokenInstructionDataTransfer,
    InputTokenDataWithContext,
    TokenData,
    TokenTransferOutputData,
} from '../types';

/// TODO: refactor akin to lightsystemprogram.transfer()
export async function createTransferInstruction(
    feePayer: PublicKey,
    authority: PublicKey,
    inputStateTrees: PublicKey[],
    inputNullifierQueues: PublicKey[],
    outputStateTrees: PublicKey[],
    inputCompressedAccounts: CompressedAccountWithMerkleContext[],
    outputCompressedAccounts: TokenTransferOutputData[],
    recentStateRootIndices: number[],
    recentValidityproof: CompressedProof,
): Promise<TransactionInstruction[]> {
    const remainingAccountsMap = new Map<PublicKey, number>();
    const inputTokenDataWithContext: InputTokenDataWithContext[] = [];
    const inputTokenData: TokenData[] = [];
    const pubkeyArray = new Set<PublicKey>();
    const coder = CompressedTokenProgram.program.coder;

    /// packs, extracts data into inputTokenData and sets data.data zero
    inputStateTrees.forEach((mt, i) => {
        if (!remainingAccountsMap.has(mt)) {
            remainingAccountsMap.set(mt, remainingAccountsMap.size);
        }
        const inputCompressedAccount = inputCompressedAccounts[i];
        const tokenData: TokenData = coder.types.decode(
            'TokenData',
            Buffer.from(inputCompressedAccount.data!.data), // FIXME: handle null
        );

        inputTokenData.push(tokenData);

        if (tokenData.delegate) {
            pubkeyArray.add(tokenData.delegate);
        }
        const inputTokenDataWithContextEntry: InputTokenDataWithContext = {
            amount: tokenData.amount,
            delegateIndex: tokenData.delegate
                ? Array.from(pubkeyArray).indexOf(tokenData.delegate)
                : null,
            delegatedAmount: tokenData.delegatedAmount.eq(bn(0))
                ? null
                : tokenData.delegatedAmount,
            isNative: tokenData.isNative,
            merkleTreePubkeyIndex: remainingAccountsMap.get(mt)!,
            nullifierQueuePubkeyIndex: 0, // Will be set in the next loop
            leafIndex: inputCompressedAccount.leafIndex,
        };
        inputTokenDataWithContext.push(inputTokenDataWithContextEntry);
    });

    inputNullifierQueues.forEach((mt, i) => {
        if (!remainingAccountsMap.has(mt)) {
            remainingAccountsMap.set(mt, remainingAccountsMap.size);
        }
        inputTokenDataWithContext[i].nullifierQueuePubkeyIndex =
            remainingAccountsMap.get(mt)!;
    });

    outputStateTrees.forEach((mt, i) => {
        if (!remainingAccountsMap.has(mt)) {
            remainingAccountsMap.set(mt, remainingAccountsMap.size);
        }
    });

    const remainingAccountMetas = Array.from(remainingAccountsMap.entries())
        .sort((a, b) => a[1] - b[1])
        .map(
            ([account]): AccountMeta => ({
                pubkey: account,
                isWritable: true, // TODO: input Merkle trees should be read-only, output Merkle trees should be writable, if a Merkle tree is for in and out c-accounts it should be writable
                isSigner: false,
            }),
        );
    const staticsAccounts = defaultStaticAccountsStruct();

    const rawInputs: CompressedTokenInstructionDataTransfer = {
        mint: inputTokenData[0].mint,
        proof: recentValidityproof,
        rootIndices: recentStateRootIndices,
        inputTokenDataWithContext,
        outputCompressedAccounts,
        outputStateMerkleTreeAccountIndices: Buffer.from(
            outputStateTrees.map(mt => remainingAccountsMap.get(mt)!),
        ),
        pubkeyArray: Array.from(pubkeyArray),
        signerIsDelegate: false,
    };

    const data = CompressedTokenProgram.program.coder.types.encode(
        'CompressedTokenInstructionDataTransfer',
        rawInputs,
    );

    /// FIXME:  why are static account params optional?
    const instruction = await CompressedTokenProgram.program.methods
        .transfer(data)
        .accounts({
            feePayer: feePayer!,
            authority: authority!,
            cpiAuthorityPda: CompressedTokenProgram.deriveCpiAuthorityPda,
            compressedPdaProgram: LightSystemProgram.programId,
            registeredProgramPda: staticsAccounts.registeredProgramPda,
            noopProgram: staticsAccounts.noopProgram,
            pspAccountCompressionAuthority:
                staticsAccounts.pspAccountCompressionAuthority,
            accountCompressionProgram:
                staticsAccounts.accountCompressionProgram,
            selfProgram: CompressedTokenProgram.programId,
        })
        .remainingAccounts(remainingAccountMetas)
        .instruction();

    return [
        ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }),
        instruction,
    ];
}
