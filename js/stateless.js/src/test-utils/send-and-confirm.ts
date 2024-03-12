import {
  Connection,
  VersionedTransaction,
  TransactionConfirmationStrategy,
  SignatureResult,
  RpcResponseAndContext,
  Signer,
  TransactionInstruction,
  TransactionMessage,
  ConfirmOptions,
} from '@solana/web3.js';

/** @returns txId */
export async function sendAndConfirmTx(
  connection: Connection,
  tx: VersionedTransaction,
  confirmOptions?: ConfirmOptions,
): Promise<string> {
  const txId = await connection.sendTransaction(tx, confirmOptions);
  const { blockhash, lastValidBlockHeight } =
    await connection.getLatestBlockhash(confirmOptions?.commitment);
  const transactionConfirmationStrategy0: TransactionConfirmationStrategy = {
    signature: txId,
    blockhash,
    lastValidBlockHeight,
  };
  await connection.confirmTransaction(
    transactionConfirmationStrategy0,
    confirmOptions?.commitment || connection.commitment || 'confirmed',
  );
  return txId;
}

export async function confirmTx(
  connection: Connection,
  txId: string,
  blockHashCtx?: { blockhash: string; lastValidBlockHeight: number },
): Promise<RpcResponseAndContext<SignatureResult>> {
  if (!blockHashCtx) blockHashCtx = await connection.getLatestBlockhash();

  const transactionConfirmationStrategy: TransactionConfirmationStrategy = {
    signature: txId,
    blockhash: blockHashCtx.blockhash,
    lastValidBlockHeight: blockHashCtx.lastValidBlockHeight,
  };
  const res = await connection.confirmTransaction(
    transactionConfirmationStrategy,
    connection.commitment || 'confirmed',
  );
  return res;
}

export function buildAndSignTx(
  instructions: TransactionInstruction[],
  payer: Signer,
  blockhash: string,
  additionalSigners: Signer[] = [],
): VersionedTransaction {
  if (additionalSigners.includes(payer))
    throw new Error('payer must not be in additionalSigners');
  const allSigners = [payer, ...additionalSigners];

  const messageV0 = new TransactionMessage({
    payerKey: payer.publicKey,
    recentBlockhash: blockhash,
    instructions,
  }).compileToV0Message();

  const tx = new VersionedTransaction(messageV0);
  tx.sign(allSigners);
  return tx;
}
