import { PublicKey, TransactionInstruction } from "@solana/web3.js";
import {
  IndexedTransaction,
  fetchRecentTransactions,
  sendVersionedTransactions,
  ProviderErrorCode,
  ProviderError,
} from "@lightprotocol/zk.js";
import { getAnchorProvider, getLightProvider } from "../utils/provider";
import { RelayError, RelayErrorCode } from "../errors";

export async function sendTransaction(req: any, res: any) {
  try {
    if (!req.body.instructions)
      throw new RelayError(
        RelayErrorCode.NO_INSTRUCTIONS_PROVIDED,
        "sendTransaction",
      );
    const provider = await getLightProvider();

    if (!provider.provider)
      throw new ProviderError(
        ProviderErrorCode.ANCHOR_PROVIDER_UNDEFINED,
        "sendTransaction",
      );

    const instructions: TransactionInstruction[] = [];
    for (const instruction of req.body.instructions) {
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

    const response = await sendVersionedTransactions(
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

    if (!provider)
      throw new ProviderError(
        ProviderErrorCode.ANCHOR_PROVIDER_UNDEFINED,
        "indexedTransactions",
      );

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
