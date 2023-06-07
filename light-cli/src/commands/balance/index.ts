import { Command, Flags, ux } from "@oclif/core";
import { BN } from "@coral-xyz/anchor";
import {
  User,
  Balance,
  InboxBalance,
  Utxo,
  TOKEN_REGISTRY,
} from "@lightprotocol/zk.js";
import { CustomLoader, getUser } from "../../utils/utils";
import { PublicKey } from "@solana/web3.js";

class BalanceCommand extends Command {
  static description =
    "Retrieve the balance, inbox balance, or UTXOs for the user";

  static flags = {
    inbox: Flags.boolean({
      char: "i",
      description: "Retrieve the inbox balance",
      default: false,
    }),
    utxos: Flags.boolean({
      char: "u",
      description: "Retrieve the UTXOs",
      default: false,
    }),
    "inbox-utxos": Flags.boolean({
      char: "x",
      description: "Retrieve the inbox UTXOs",
      default: false,
    }),
    latest: Flags.boolean({
      char: "l",
      description: "Retrieve the latest balance, inbox balance, or UTXOs",
      default: true,
    }),
    verbose: Flags.boolean({
      char: "v",
      description: "Print SPL token stringified UTXOs",
      dependsOn: ['token'],
      default: false,
    }),
    table: Flags.boolean({
      char: "T",
      description: "Print balance in table format",
      default: false,
    }),
    token: Flags.string({
      char: "t",
      description: "The SPL token symbol",
      default: "SOL",
      required: false
    }),
  };

  protected finally(_: Error | undefined): Promise<any> {
    process.exit();
  }

  static examples = [
    "$ light balance --inbox",
    "$ light balance --utxos --inbox",
    "$ light balance --inbox-utxos",
    "$ light balance --latest=false",
    "$ light balance --token USDC --verbose"
  ];

  async run() {
    const { flags } = await this.parse(BalanceCommand);
    const { inbox, utxos, latest, verbose, table } = flags;
    const token = flags.token ?? "none";
    const inboxUtxos = flags["inbox-utxos"];

    const loader = new CustomLoader("Retrieving balance...");
    loader.start();

    const user: User = await getUser();
    //const tokenCtx = TOKEN_REGISTRY.get(token.toUpperCase());

    const balances = await user.getBalance(latest);
    
    if(token === "SOL") {
      this.log("\n--- Balance ---");
      this.logMainBalances(balances, table);
      this.log('\n');
    }
    else {
      this.logTokenBalance(balances, token.toString());
    } 

    try {
      if (inbox) {
        const inboxBalances = await user.getUtxoInbox(latest);
        this.log("--- Inbox Balance ---");
        this.logMainBalances(inboxBalances);
        this.log("\n");      
      }
      if (utxos) {
        const utxos = await user.getAllUtxos();
        this.logUTXOs(utxos);
      }
      if (inboxUtxos) {
        const result = await user.getUtxoInbox();
        const utxos: Utxo[] = [];
        for (const iterator of result.tokenBalances.values()) {
          iterator.utxos.forEach((value) => {
            utxos.push(value);
          });
        }
        this.logUTXOs(utxos);
      }
      
      loader.stop();
    } catch (error) {
      loader.stop();
      this.warn(error as Error);
      this.error(`Error retrieving balance, inbox balance, or UTXOs: ${error}`);
    }
  }

  private logMainBalances(balances: Balance | InboxBalance, table?: boolean) {
    let tableData = [];
    for (const tokenBalance of balances.tokenBalances) {
      //this.log('token balance: ', tokenBalance);
      let _token = tokenBalance[1].tokenData.symbol;
      let decimals = tokenBalance[1].tokenData.decimals;
      let balance: BN = _token === "SOL" 
        ? tokenBalance[1].totalBalanceSol.div(decimals)
        : tokenBalance[1].totalBalanceSpl.div(decimals);
      let utxoNumber = tokenBalance[1].utxos.size;
      tableData.push({
        token: _token,
        balance: balance.toNumber(),
        utxos: utxoNumber
      })
      this.log(`${_token} Balance: ${balance} => ${utxoNumber} utxos`);
    }
    if (table) {
      this.log("\n")
      ux.table(tableData, {
      token: {},
      balance: {},
      utxos: {},
      })
    } 
  }

  private logTokenBalance(balances: Balance | InboxBalance, token: string) {
    for (const tokenBalance of balances.tokenBalances) {
      let _token = tokenBalance[1].tokenData.symbol;
      if (token === _token) {
        let decimals = tokenBalance[1].tokenData.decimals;
        let balance = token === "SOL" 
          ? tokenBalance[1].totalBalanceSol.div(decimals)
          : tokenBalance[1].totalBalanceSpl.div(decimals);
        let utxoNumber = tokenBalance[1].utxos.size;
        this.log(`\n${_token} Balance: ${balance} => ${utxoNumber} utxos\n`);
        break
      }
    }
  }

  private async logBalance(
    balance: Balance,
    verbose: boolean,
    token: PublicKey
  ) {
    this.log("\n--- Balance ---");

    const tokenBalance = balance.tokenBalances.get(token.toString());
    
    if (tokenBalance && tokenBalance?.tokenData.symbol !== "SOL") {
      this.log(`\x1b[34mSPL Token\x1b[0m:    ${tokenBalance!.tokenData.symbol}`);
      this.log(`\x1b[34mToken Amount\x1b[0m: ${tokenBalance!.totalBalanceSpl.toNumber() / 100}`);
      this.log(`\x1b[34mMint\x1b[0m:         ${tokenBalance?.tokenData.mint.toString()}`);
      this.log(`\x1b[34mSOL Amount\x1b[0m:   ${this.convertToSol(tokenBalance!.totalBalanceSol)}`);
      this.log(`\x1b[34mUTXO Number\x1b[0m:  ${tokenBalance!.utxos.size}`);
      this.log("\nUTXOS:");
      for (const iterator of tokenBalance?.utxos.values()!) {
        this.log(
          `\t${
            tokenBalance!.tokenData.symbol
          }: ${iterator.amounts[1].toNumber() / 100 }\n\tSOL: ${this.convertToSol(iterator.amounts[0])}\n\tCommitmentHash: ${
            iterator._commitment
          }`
        );
        if (verbose) {
          const utxo = await iterator.toString();
          this.log("\tStringified UTXO: ", utxo);
        }
      }
    } else {
      let totalSolBalance = this.convertToSol(balance.totalSolBalance);
      this.log("Total Shielded SOL Balance:", totalSolBalance.toString());
    }

    this.log("----------------");
  }

  private logUTXOs(utxos: Utxo[]) {
    this.log("\n--- UTXOs ---");
    for (const utxo of utxos) {
      this.log("UTXO:");
      this.log(`Amount: ${utxo.amounts}`);
      this.log(`Asset: ${utxo.assets}`);
      this.log(`Commitment: ${utxo._commitment}`);
      this.log(`Index: ${utxo.index}`);
    }
    this.log("----------------");
  }
  // apply to all flags
  private convertToSol(amount: BN): number {
    const SOL_DECIMALS = new BN(1_000_000_000);
    return amount.div(SOL_DECIMALS).toNumber()
  }
}

module.exports = BalanceCommand;
