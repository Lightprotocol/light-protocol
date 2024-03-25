import {
  Commitment,
  Connection,
  PublicKey,
  TransactionConfirmationStrategy,
} from '@solana/web3.js';

export async function airdropSol({
  connection,
  lamports,
  recipientPublicKey,
}: {
  connection: Connection;
  lamports: number;
  recipientPublicKey: PublicKey;
}) {
  const txHash = await connection.requestAirdrop(recipientPublicKey, lamports);
  await confirmTransaction(connection, txHash);
  return txHash;
}

export async function confirmTransaction(
  connection: Connection,
  signature: string,
  confirmation: Commitment = 'confirmed',
) {
  const latestBlockHash = await connection.getLatestBlockhash(confirmation);
  const strategy: TransactionConfirmationStrategy = {
    signature: signature.toString(),
    lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
    blockhash: latestBlockHash.blockhash,
  };
  return await connection.confirmTransaction(strategy, confirmation);
}
