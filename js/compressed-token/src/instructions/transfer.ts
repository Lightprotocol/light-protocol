import {
  UtxoWithBlinding,
  MockProof,
  InUtxoTuple,
  defaultStaticAccountsStruct,
  LightSystemProgram,
} from '@lightprotocol/stateless.js';
import {
  PublicKey,
  TransactionInstruction,
  AccountMeta,
} from '@solana/web3.js';
import { BN } from '@coral-xyz/anchor';
import { CompressedTokenProgram } from '../program';

// Implement refactor
export type TokenTransferOutUtxo = {
  owner: PublicKey;
  amount: BN;
  lamports: BN | null;
  index_mt_account: number;
};

enum AccountState {
  Uninitialized,
  Initialized,
  Frozen,
}

// TODO: beet -> change property names to camelCase
export type TokenTlvData = {
  /// The mint associated with this account
  mint: PublicKey;
  /// The owner of this account.
  owner: PublicKey;
  /// The amount of tokens this account holds.
  amount: number;
  /// If `delegate` is `Some` then `delegated_amount` represents
  /// the amount authorized by the delegate
  delegate?: PublicKey;
  /// The account's state
  state: AccountState;
  /// If is_some, this is a native token, and the value logs the rent-exempt
  /// reserve. An Account is required to be rent-exempt, so the value is
  /// used by the Processor to ensure that wrapped SOL accounts do not
  /// drop below this threshold.
  is_native?: number;
  /// The amount delegated
  delegated_amount: number;
  // TODO: validate that we don't need close authority
  // /// Optional authority to close the account.
  // close_authority?: PublicKey,
};

// NOTE: this is currently akin to createExecuteCompressedInstruction on-chain
export async function createTransferInstruction(
  feePayer: PublicKey,
  authority: PublicKey,
  inUtxoMerkleTreePubkeys: PublicKey[],
  nullifierArrayPubkeys: PublicKey[],
  outUtxoMerkleTreePubkeys: PublicKey[],
  inUtxos: UtxoWithBlinding[],
  outUtxos: TokenTransferOutUtxo[], // tlv missing
  rootIndices: number[],
  proof: MockProof,
): Promise<TransactionInstruction> {
  const outputUtxos = outUtxos.map((utxo) => ({ ...utxo }));
  const remainingAccountsMap = new Map<PublicKey, number>();
  const inUtxosWithIndex: InUtxoTuple[] = [];
  const inUtxoTlvData: TokenTlvData[] = [];

  const coder = CompressedTokenProgram.program.coder;

  inUtxoMerkleTreePubkeys.forEach((mt, i) => {
    if (!remainingAccountsMap.has(mt)) {
      remainingAccountsMap.set(mt, remainingAccountsMap.size);
    }
    const inUtxo = inUtxos[i];
    const tokenTlvData: TokenTlvData = coder.types.decode(
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
  const data = (
    await CompressedTokenProgram.program.coder.accounts.encode(
      'InstructionDataTransferClient',
      rawInputs,
    )
  ).subarray(8);
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
