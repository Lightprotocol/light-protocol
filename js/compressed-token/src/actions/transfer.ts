import {
  ConfirmOptions,
  Connection,
  PublicKey,
  Signer,
  TransactionSignature,
  ComputeBudgetProgram,
  SystemProgram,
} from '@solana/web3.js';
import { CompressedTokenProgram } from '../program';
import {
  CompressedProof_IdlType,
  TlvDataElement_IdlType,
  Utxo_IdlType,
  bn,
  defaultTestStateTreeAccounts,
  sendAndConfirmTx,
} from '@lightprotocol/stateless.js';
import { buildAndSignTx } from '@lightprotocol/stateless.js';
import { BN } from '@coral-xyz/anchor';
import { createTransferInstruction } from '../instructions';
import { TokenTlvData_IdlType, TokenTransferOutUtxo_IdlType } from '../types';
import { getSigners } from './mint-to';

/**
 * Transfer compressed tokens from one owner to another
 *
 * @param connection     Connection to use
 * @param payer          Payer of the transaction fees
 * @param mint           Mint of the compressed token
 * @param amount         Number of tokens to transfer
 * @param owner          Owner of the compressed tokens
 * @param toAddress      Destination address of the recipient
 * @param merkleTree     State tree account that the compressed tokens should be
 *                       inserted into. Defaults to the default state tree account.
 * @param multiSigners   Signing accounts if `currentOwner` is a multisig
 * @param confirmOptions Options for confirming the transaction
 *
 *
 * @return Signature of the confirmed transaction
 */
export async function transfer(
  connection: Connection,
  payer: Signer,
  mint: PublicKey,
  amount: number | BN,
  owner: Signer | PublicKey,
  toAddress: PublicKey,
  merkleTree: PublicKey = defaultTestStateTreeAccounts().merkleTree,
  multiSigners: Signer[] = [],
  confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
  const keys = defaultTestStateTreeAccounts();
  const queue = keys.nullifierQueue; /// FIXME: Should fetch or provide

  // returns signers = [currentOwner] here
  const [currentOwnerPublicKey, signers] = getSigners(owner, multiSigners);

  /// TODO: don't mock input state + proof.
  /// Also: refactor createTransferInstruction
  const tlv: TokenTlvData_IdlType = {
    mint: mint,
    owner: currentOwnerPublicKey,
    amount: bn(amount).add(bn(42)), // +42
    delegate: null,
    state: 1,
    isNative: null,
    delegatedAmount: bn(0),
  };

  const tlvData = CompressedTokenProgram.program.coder.types.encode(
    'TokenTlvDataClient',
    tlv,
  );

  const tlvDataElement: TlvDataElement_IdlType = {
    discriminator: Array(8).fill(2),
    owner: CompressedTokenProgram.programId, // tok
    data: Uint8Array.from(tlvData),
    dataHash: Array(32).fill(0), // mock
  };

  const inUtxo: Utxo_IdlType = {
    owner: CompressedTokenProgram.programId,
    blinding: Array(32).fill(0),
    lamports: new BN(0),
    data: { tlvElements: [tlvDataElement] },
    address: null,
  };

  /// Create output utxos

  const changeUtxo: TokenTransferOutUtxo_IdlType = {
    amount: bn(42), // mocked input state value
    owner: currentOwnerPublicKey, /// FIXME: on-chain must accept tokenprogramowner
    lamports: null,
    index_mt_account: 0,
  };

  const recipientOutUtxo: TokenTransferOutUtxo_IdlType = {
    amount: bn(amount),
    owner: toAddress,
    lamports: null,
    index_mt_account: 0,
  };

  const proof_mock: CompressedProof_IdlType = {
    a: Array.from({ length: 32 }, () => 0),
    b: Array.from({ length: 64 }, () => 0),
    c: Array.from({ length: 32 }, () => 0),
  };

  const ix = await createTransferInstruction(
    payer.publicKey,
    currentOwnerPublicKey,
    [merkleTree],
    [queue],
    [merkleTree, merkleTree],
    [inUtxo],
    [recipientOutUtxo, changeUtxo],
    [0], // input state root indices
    proof_mock,
  );

  const ixs = [
    ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
    ix,
  ];
  const { blockhash } = await connection.getLatestBlockhash();

  /// TODO: find more elegant solution for this.
  /// Probably buildAndSignTx should just dedupe by itself
  const filteredSigners = currentOwnerPublicKey.equals(payer.publicKey)
    ? signers.filter(
        (signer) => !signer.publicKey.equals(currentOwnerPublicKey),
      )
    : [...signers];
  console.log('filteredSigners', filteredSigners);
  const signedTx = buildAndSignTx(ixs, payer, blockhash, filteredSigners);
  const txId = await sendAndConfirmTx(connection, signedTx, confirmOptions);

  return txId;
}
