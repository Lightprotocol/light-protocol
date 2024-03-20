import {
  CompressedProof,
  defaultStaticAccountsStruct,
  CompressedAccountWithMerkleContext,
  LightSystemProgram,
  PackedCompressedAccountWithMerkleContext,
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
  const packedInputCompressedAccountsWithMerkleContext: PackedCompressedAccountWithMerkleContext[] =
    [];
  const inputTokenData: TokenData[] = [];

  const coder = CompressedTokenProgram.program.coder;

  /// packs, also sets data zero, so if they have data they need to extract it beforehand
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
    inputCompressedAccount.data = null; // FIXME
    packedInputCompressedAccountsWithMerkleContext.push({
      compressedAccount: inputCompressedAccount,
      indexMtAccount: remainingAccountsMap.get(mt)!,
      indexNullifierArrayAccount: 0, // Will be set in the next loop
      leafIndex: inputCompressedAccount.leafIndex,
    });
  });

  inputNullifierQueues.forEach((mt, i) => {
    if (!remainingAccountsMap.has(mt)) {
      remainingAccountsMap.set(mt, remainingAccountsMap.size);
    }
    packedInputCompressedAccountsWithMerkleContext[
      i
    ].indexNullifierArrayAccount = remainingAccountsMap.get(mt)!;
  });

  outputStateTrees.forEach((mt, i) => {
    if (!remainingAccountsMap.has(mt)) {
      remainingAccountsMap.set(mt, remainingAccountsMap.size);
    }
    outputCompressedAccounts[i].index_mt_account =
      remainingAccountsMap.get(mt)!;
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
    proof: recentValidityproof,
    rootIndices: recentStateRootIndices,
    inputCompressedAccountsWithMerkleContext:
      packedInputCompressedAccountsWithMerkleContext,
    inputTokenData,
    outputCompressedAccounts,
    outputStateMerkleTreeAccountIndices: Buffer.from(
      outputStateTrees.map(mt => remainingAccountsMap.get(mt)!),
    ),
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
      accountCompressionProgram: staticsAccounts.accountCompressionProgram,
      selfProgram: CompressedTokenProgram.programId,
    })
    .remainingAccounts(remainingAccountMetas)
    .instruction();

  return [
    ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }),
    instruction,
  ];
}
