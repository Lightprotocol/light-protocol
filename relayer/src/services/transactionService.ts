import {
  fetchRecentTransactions,
  ProviderErrorCode,
  ProviderError,
} from "@lightprotocol/zk.js";
import { getAnchorProvider } from "../utils/provider";
import { RelayerIndexedTransaction } from "@lightprotocol/zk.js";

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
      (trx: RelayerIndexedTransaction) => {
        return {
          IDs: trx.IDs,
          merkleTreePublicKey: trx.merkleTreePublicKey.toString(),
          transaction: {
            ...trx.transaction,
            publicAmountSol: trx.transaction.publicAmountSol.toString(),
            publicAmountSpl: trx.transaction.publicAmountSpl.toString(),
            changeSolAmount: trx.transaction.changeSolAmount.toString(),
            relayerFee: trx.transaction.relayerFee.toString(),
            firstLeafIndex: trx.transaction.firstLeafIndex.toString(),
          },
        };
      },
    );

    return res.status(200).json({ data: stringifiedIndexedTransactions });
  } catch (error) {
    return res.status(500).json({ status: "error", message: error.message });
  }
}
