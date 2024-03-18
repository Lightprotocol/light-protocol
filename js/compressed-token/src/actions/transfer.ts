import {
  ConfirmOptions,
  Connection,
  PublicKey,
  Signer,
  TransactionSignature,
  ComputeBudgetProgram,
} from '@solana/web3.js';
import {
  CompressedProof_IdlType,
  bn,
  defaultTestStateTreeAccounts,
  sendAndConfirmTx,
  getMockRpc,
} from '@lightprotocol/stateless.js';
import { buildAndSignTx } from '@lightprotocol/stateless.js';
import { BN } from '@coral-xyz/anchor';
import { createTransferInstruction } from '../instructions';
import { TokenTransferOutUtxo_IdlType } from '../types';
import { getSigners } from './mint-to';
import {
  UtxoWithParsedTokenTlvData,
  getCompressedTokenAccountsFromMockRpc,
} from '../token-serde';

/**
 * @internal
 *
 * Selects the minimal number of compressed token accounts for a transfer
 * 1. Sorts the accounts by amount in descending order
 * 2. Accumulates the amount until it is greater than or equal to the transfer
 *    amount
 */
function pickMinCompressedTokenAccountsForTransfer(
  accounts: UtxoWithParsedTokenTlvData[],
  transferAmount: BN,
): UtxoWithParsedTokenTlvData[] {
  let accumulatedAmount = bn(0);
  const selectedAccounts = [];
  accounts.sort((a, b) => b.parsed.amount.cmp(a.parsed.amount));
  for (const account of accounts) {
    if (accumulatedAmount.gte(bn(transferAmount))) break;
    accumulatedAmount = accumulatedAmount.add(account.parsed.amount);
    selectedAccounts.push(account);
  }
  if (accumulatedAmount.lt(bn(transferAmount))) {
    throw new Error('Not enough balance for transfer');
  }
  return selectedAccounts;
}
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
  const queue = keys.nullifierQueue; /// FIXME: Should fetch or pass
  const [currentOwnerPublicKey, signers] = getSigners(owner, multiSigners);

  const compressedTokenAccounts = await getCompressedTokenAccountsFromMockRpc(
    connection,
    currentOwnerPublicKey,
    mint,
  );

  const inUtxos = pickMinCompressedTokenAccountsForTransfer(
    compressedTokenAccounts,
    bn(amount),
  );

  /// Create output utxos
  const changeAmount = inUtxos
    .reduce((acc, utxo) => acc.add(utxo.parsed.amount), bn(0))
    .sub(bn(amount));

  /// We don't send lamports and don't have rent
  const changeLamportsAmount = inUtxos.reduce(
    (acc, utxo) => acc.add(utxo.utxo.lamports),
    // TODO: add optional rent
    bn(0),
  );

  const changeUtxo: TokenTransferOutUtxo_IdlType = {
    amount: changeAmount,
    owner: currentOwnerPublicKey,
    lamports: changeLamportsAmount.gt(bn(0)) ? changeLamportsAmount : null,
    index_mt_account: 0, // FIXME: dynamic!
  };

  const recipientOutUtxo: TokenTransferOutUtxo_IdlType = {
    amount: bn(amount),
    owner: toAddress,
    lamports: null,
    index_mt_account: 0, // FIXME: dynamic!
  };

  // TODO: replace with actual proof!
  // const proof_mock: CompressedProof_IdlType = {
  //   a: Array.from({ length: 32 }, () => 0),
  //   b: Array.from({ length: 64 }, () => 0),
  //   c: Array.from({ length: 32 }, () => 0),
  // };
  const rpc = await getMockRpc(connection);

  const proof = await rpc.getValidityProof(
    inUtxos.map((utxo) => utxo.merkleContext.hash as BN),
  );

  const ix = await createTransferInstruction(
    payer.publicKey,
    currentOwnerPublicKey,
    [merkleTree],
    [queue],
    [merkleTree, merkleTree],
    inUtxos.map((utxo) => utxo.utxo),
    [recipientOutUtxo, changeUtxo],
    // TODO: replace with actual recent state root index!
    // This will only work with sequential state updates and no cranking!
    inUtxos.map((utxo) => Number(utxo.merkleContext?.leafIndex)), // input state root indices
    proof.compressedProof,
  );

  const ixs = [
    // TODO: adjust CU down to min!
    ComputeBudgetProgram.setComputeUnitLimit({ units: 300_000 }),
    ix,
  ];
  const { blockhash } = await connection.getLatestBlockhash();

  // TODO: find more elegant solution for this.
  // Probably buildAndSignTx should just dedupe by itself
  const filteredSigners = currentOwnerPublicKey.equals(payer.publicKey)
    ? signers.filter(
        (signer) => !signer.publicKey.equals(currentOwnerPublicKey),
      )
    : [...signers];
  const signedTx = buildAndSignTx(ixs, payer, blockhash, filteredSigners);
  const txId = await sendAndConfirmTx(connection, signedTx, confirmOptions);

  return txId;
}
