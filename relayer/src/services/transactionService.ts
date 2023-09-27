import { PublicKey, TransactionInstruction } from "@solana/web3.js";
import {
  IndexedTransaction,
  fetchRecentTransactions,
  sendVersionedTransactions,
} from "@lightprotocol/zk.js";
import { getAnchorProvider, getLightProvider } from "../utils/provider";

export async function sendTransaction(req: any, res: any) {
  try {
    if (!req.body.instructions) throw new Error("No instructions provided");
    const provider = await getLightProvider();

    if (!provider.provider) throw new Error("no provider set");

    let instructions: TransactionInstruction[] = [];
    for (let instruction of req.body.instructions) {
      const accounts = instruction.keys.map((key: any) => {
        return {
          pubkey: new PublicKey(key.pubkey),
          isWritable: key.isWritable,
          isSigner: key.isSigner,
        };
      });
      const newInstruction = new TransactionInstruction({
        keys: accounts,
        programId: new PublicKey(instruction.programId),
        data: Buffer.from(instruction.data),
      });
      instructions.push(newInstruction);
    }

    var response = await sendVersionedTransactions(
      instructions,
      provider.provider.connection,
      provider.lookUpTables.versionedTransactionLookupTable,
      provider.wallet,
    );
    return res
      .status(200)
      .json({ data: { transactionStatus: "confirmed", ...response } });
  } catch (error) {
    return res.status(500).json({ status: "error", message: error.message });
  }
}

export async function indexedTransactions(_req: any, res: any) {
  try {
    const provider = await getAnchorProvider();

    if (!provider) throw new Error("no provider set");

    const { transactions: indexedTransactions } = await fetchRecentTransactions(
      {
        connection: provider.connection,
        batchOptions: {
          limit: 5000,
        },
      },
    );

    const stringifiedIndexedTransactions = indexedTransactions.map(
      (trx: IndexedTransaction) => {
        return {
          ...trx,
          publicAmountSol: trx.publicAmountSol.toString(),
          publicAmountSpl: trx.publicAmountSpl.toString(),
          changeSolAmount: trx.changeSolAmount.toString(),
          relayerFee: trx.relayerFee.toString(),
          firstLeafIndex: trx.firstLeafIndex.toString(),
        };
      },
    );

    return res.status(200).json({ data: stringifiedIndexedTransactions });
  } catch (error) {
    return res.status(500).json({ status: "error", message: error.message });
  }
}
