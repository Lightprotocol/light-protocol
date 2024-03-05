import {
  Connection,
  VersionedTransaction,
  TransactionConfirmationStrategy,
  SignatureResult,
  RpcResponseAndContext,
  Transaction,
} from "@solana/web3.js";

export async function sendAndConfirmTx(
  connection: Connection,
  tx: VersionedTransaction
) {
  const txId = await connection.sendTransaction(tx);
  const { blockhash, lastValidBlockHeight } =
    await connection.getLatestBlockhash();
  const transactionConfirmationStrategy0: TransactionConfirmationStrategy = {
    signature: txId,
    blockhash,
    lastValidBlockHeight,
  };
  const res = await connection.confirmTransaction(
    transactionConfirmationStrategy0,
    connection.commitment
  );
  return res;
}

export async function confirmTx(
  connection: Connection,
  txId: string,
  blockHashCtx?: { blockhash: string; lastValidBlockHeight: number }
): Promise<RpcResponseAndContext<SignatureResult>> {
  if (!blockHashCtx) blockHashCtx = await connection.getLatestBlockhash();

  const transactionConfirmationStrategy: TransactionConfirmationStrategy = {
    signature: txId,
    blockhash: blockHashCtx.blockhash,
    lastValidBlockHeight: blockHashCtx.lastValidBlockHeight,
  };
  const res = await connection.confirmTransaction(
    transactionConfirmationStrategy,
    connection.commitment || "confirmed"
  );
  return res;
}
