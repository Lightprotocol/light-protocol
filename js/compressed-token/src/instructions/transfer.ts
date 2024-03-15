import {
  CompressedProof_IdlType,
  defaultStaticAccountsStruct,
  InUtxoTuple_IdlType,
  LightSystemProgram,
  Utxo_IdlType,
} from '@lightprotocol/stateless.js';
import {
  PublicKey,
  TransactionInstruction,
  AccountMeta,
} from '@solana/web3.js';
import { CompressedTokenProgram } from '../program';
import { TokenTlvData_IdlType, TokenTransferOutUtxo_IdlType } from '../types';

// NOTE: this is currently akin to createExecuteCompressedInstruction on-chain
export async function createTransferInstruction(
  feePayer: PublicKey,
  authority: PublicKey,
  inUtxoMerkleTreePubkeys: PublicKey[],
  nullifierArrayPubkeys: PublicKey[],
  outUtxoMerkleTreePubkeys: PublicKey[],
  inUtxos: Utxo_IdlType[],
  outUtxos: TokenTransferOutUtxo_IdlType[], // tlv missing
  rootIndices: number[],
  proof: CompressedProof_IdlType,
): Promise<TransactionInstruction> {
  const outputUtxos = outUtxos.map((utxo) => ({ ...utxo }));
  const remainingAccountsMap = new Map<PublicKey, number>();
  const inUtxosWithIndex: InUtxoTuple_IdlType[] = [];
  const inUtxoTlvData: TokenTlvData_IdlType[] = [];

  const coder = CompressedTokenProgram.program.coder;

  inUtxoMerkleTreePubkeys.forEach((mt, i) => {
    if (!remainingAccountsMap.has(mt)) {
      remainingAccountsMap.set(mt, remainingAccountsMap.size);
    }
    const inUtxo = inUtxos[i];
    const tokenTlvData: TokenTlvData_IdlType = coder.types.decode(
      'TokenTlvData',
      Buffer.from(inUtxo.data!.tlvElements[0].data), // FIXME: handle null
    );

    inUtxoTlvData.push(tokenTlvData);
    inUtxo.data = null;
    inUtxosWithIndex.push({
      inUtxo,
      indexMtAccount: remainingAccountsMap.get(mt)!,
      indexNullifierArrayAccount: 0, // Will be set in the next loop
    });
  });

  nullifierArrayPubkeys.forEach((mt, i) => {
    if (!remainingAccountsMap.has(mt)) {
      remainingAccountsMap.set(mt, remainingAccountsMap.size);
    }
    inUtxosWithIndex[i].indexNullifierArrayAccount =
      remainingAccountsMap.get(mt)!;
  });

  outUtxoMerkleTreePubkeys.forEach((mt, i) => {
    if (!remainingAccountsMap.has(mt)) {
      remainingAccountsMap.set(mt, remainingAccountsMap.size);
    }
    outputUtxos[i].index_mt_account = remainingAccountsMap.get(mt)!;
  });

  const remainingAccountMetas = Array.from(remainingAccountsMap.entries())
    .sort((a, b) => a[1] - b[1])
    .map(
      ([account]): AccountMeta => ({
        pubkey: account,
        isWritable: true, // TODO: input Merkle trees should be read-only, output Merkle trees should be writable, if a Merkle tree is for in and out utxos it should be writable
        isSigner: false,
      }),
    );
  const staticsAccounts = defaultStaticAccountsStruct();

  const rawInputs = {
    proof,
    rootIndices,
    inUtxos: inUtxosWithIndex,
    inTlvData: inUtxoTlvData,
    outUtxos,
  };
  /// TODO: check!
  const data = CompressedTokenProgram.program.coder.types.encode(
    'InstructionDataTransfer',
    rawInputs,
  );
  /// FIXME:  why are static account params optional?
  const instruction = await CompressedTokenProgram.program.methods
    .transfer(data)
    .accounts({
      feePayer: feePayer!,
      authority: authority!,
      cpiAuthorityPda: CompressedTokenProgram.cpiAuthorityPda,
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

  return instruction;
}
