import {
  IndexedTransaction,
  fetchRecentTransactions,
  ProviderErrorCode,
  ProviderError,
} from "@lightprotocol/zk.js";
import { getAnchorProvider } from "../utils/provider";

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
