import { Command, ux } from "@oclif/core";
import { BN } from "@coral-xyz/anchor";
import {
  ParsedIndexedTransaction,
  SOL_DECIMALS,
  convertAndComputeDecimals,
} from "@lightprotocol/zk.js";
import { CustomLoader, getUser, standardFlags } from "../../utils";

type TransactionHistory = {
  TransactionNumber: number;
  Timestamp: string;
  Signer: string;
  Signature: string;
  From: string;
  To: string;
  RelayerRecipientSOL: string;
  Type: string;
  PublicAmountSOL: number;
  PublicAmountSPL: number;
  RelayerFeeSOL: number;
};
class TransactionHistoryCommand extends Command {
  static description = "Show user transaction history";
  static flags = {
    ...standardFlags,
  };

  async run() {
    const { flags } = await this.parse(TransactionHistoryCommand);

    const loader = new CustomLoader("Retrieving user transaction history...");
    loader.start();

    try {
      this.log("\n");

      const user = await getUser({
        skipFetchBalance: flags["skipFetchBalance"],
        localTestRelayer: flags["localTestRelayer"],
      });
      const transactions: ParsedIndexedTransaction[] =
        await user.getTransactionHistory(false);

      transactions.reverse().forEach((transaction, index) => {
        // TODO: add mint to indexed transactions
        const splDecimals = new BN(100);
        const date = new Date(transaction.blockTime);
        const transactionHistory: TransactionHistory = {
          TransactionNumber: index + 1,
          Timestamp: date.toString(),
          Type: `\x1b[32m${transaction.type}\x1b[0m`,
          PublicAmountSOL: convertAndComputeDecimals(
            transaction.publicAmountSol,
            SOL_DECIMALS,
          ).toNumber(),
          PublicAmountSPL: convertAndComputeDecimals(
            transaction.publicAmountSpl,
            splDecimals,
          ).toNumber(),
          From: transaction.from.toString(),
          To: transaction.to.toString(),
          RelayerRecipientSOL: transaction.relayerRecipientSol.toString(),
          RelayerFeeSOL: convertAndComputeDecimals(
            transaction.relayerFee,
            SOL_DECIMALS,
          ).toNumber(),
          Signer: transaction.signer.toString(),
          Signature: transaction.signature,
        };

        switch (transaction.type) {
          case "SHIELD":
            this.logTransaction(transactionHistory, [
              "RelayerFee",
              "RelayerFeeSOL",
              "To",
              "RelayerRecipientSOL",
              "From",
            ]);
            break;
          case "UNSHIELD":
            this.logTransaction(transactionHistory, [
              "From",
              "To",
              "RelayerFee",
            ]);
            break;
          case "TRANSFER":
            this.logTransaction(transactionHistory, [
              "PublicAmountSOL",
              "PublicAmountSPL",
              "From",
              "To",
            ]);
            break;
          default:
            this.logTransaction(transactionHistory); // If none of the cases match, it logs all keys and values
            break;
        }
      });
      loader.stop(false);
    } catch (error) {
      this.error(`\nFailed to retrieve transaction history!\n${error}`);
    }
  }

  private logTransaction(
    transaction: TransactionHistory,
    ignoreKeys: string[] = [],
  ): void {
    const tableData: any[] = [];
    const actionCheck =
      transaction.Type == `\x1b[32mTRANSFER\x1b[0m` ||
      transaction.Type == `\x1b[32mUNSHIELD\x1b[0m`;
    Object.keys(transaction).forEach((key) => {
      if (!ignoreKeys.includes(key)) {
        // Transform the key from camel case to separate words, each starting with a capital letter.
        const formattedKey = key
          .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
          .replace(/([A-Z])([A-Z][a-z])/g, "$1 $2");
        let capitalizedKey = formattedKey
          .split(" ")
          .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
          .join(" ");
        const value = transaction[key as keyof TransactionHistory];
        if (capitalizedKey === "Transaction Number")
          capitalizedKey = "Transaction Number   ";
        if (capitalizedKey === "Signer" && actionCheck) {
          capitalizedKey = "Relayer Signer";
        }
        tableData.push({ prop: `\x1b[34m${capitalizedKey}\x1b[0m`, value });
      }
    });
    ux.table(tableData, {
      prop: { header: "" },
      value: { header: "" },
    });
  }
}

export default TransactionHistoryCommand;
