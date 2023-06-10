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
    token: Flags.string({
      char: "t",
      description: "The SPL token symbol",
      default: "SOL",
      parse: async (token) => token.toUpperCase(), 
      required: false
    }),
    "inbox-utxos": Flags.boolean({
      char: "x",
      description: "Retrieve the inbox UTXOs",
      default: false,
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
    const { inbox, utxos, latest, verbose, token } = flags;
    const inboxUtxos = flags["inbox-utxos"];

    const loader = new CustomLoader("Retrieving balance...");
    loader.start();

    const user: User = await getUser();
    const tokenCtx = TOKEN_REGISTRY.get(token);

    const balances = await user.getBalance(latest);
    
    const PURPLE = "\x1b[35m%s\x1b[0m";

    if(token === "SOL") {
      
      this.log(PURPLE, "\n--- Shielded Balance ---\n");
      this.logMainBalances(balances);
      this.log('\n');
    }
    else {
      const inboxBalances = await user.getUtxoInbox(latest);
      this.log('\n');
      this.logTokenBalance(balances, inboxBalances, token);
    } 

    try {
      if (inbox && token === "SOL") {
        const inboxBalances = await user.getUtxoInbox(latest);
        this.log(PURPLE, "--- Inbox Balance ---\n");
        this.logMainBalances(inboxBalances);
        this.log("\n");      
      }
      if (utxos) {
        const utxos = await user.getAllUtxos();
        //this.logUTXOs(utxos);
        this.logTokenUtxos(balances, tokenCtx!.mint)
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

  private logMainBalances(balances: Balance | InboxBalance) {
    let tableData = [];
    for (const tokenBalance of balances.tokenBalances) {
      //this.log('token balance: ', tokenBalance);
      let _token = tokenBalance[1].tokenData.symbol;
      let decimals = tokenBalance[1].tokenData.decimals;
      let balance: BN = _token === "SOL" 
        ? tokenBalance[1].totalBalanceSol / decimals
        : tokenBalance[1].totalBalanceSpl / decimals;
      let utxoNumber = tokenBalance[1].utxos.size;
      tableData.push({
        token: _token,
        balance: balance,
        utxos: utxoNumber
      })
    } 
    ux.table(tableData, {
      token: {},
      balance: {},
      utxos: {},
    })
  }

  private fetchTokenBalance(balances: Balance | InboxBalance, token: string, inbox=false) {
    for (const tokenBalance of balances.tokenBalances) {
      let _token = tokenBalance[1].tokenData.symbol;
      if (token === _token) {
        let decimals = tokenBalance[1].tokenData.decimals;
        let balance = token === "SOL" 
          ? tokenBalance[1].totalBalanceSol / decimals
          : tokenBalance[1].totalBalanceSpl / decimals;
        let utxoNumber = tokenBalance[1].utxos.size;
        let type = inbox ? 'inbox' : 'normal';
        return {
          token: _token,
          amount: balance,
          balance: type,
          utxos: utxoNumber
        }
      }
    }
  }

  private logTokenUtxos(balance: Balance, token: PublicKey, verbose=false) {
    const tokenBalance = balance.tokenBalances.get(token.toString());
    
    if (tokenBalance && tokenBalance?.tokenData.symbol !== "SOL") {
      let i=0;
      let decimals = tokenBalance.tokenData.decimals;
      let tables = [];
      for (const iterator of tokenBalance?.utxos.values()!) {
        i++;
        let amountSpl = iterator.amounts[1] / decimals;
        let amountSol = this.convertToSol(iterator.amounts[0]);
        let symbol = tokenBalance.tokenData.symbol;
        let mint = tokenBalance.tokenData.mint.toString();
        let commitmentHash = iterator._commitment;
        let tableData = [
          {key: 'No', value: i},
          {key: 'Token', value: symbol},
          {key: 'Amount SOL', value: amountSol},
          {key: 'Amount SPL', value: amountSpl},
          {key: 'Mint', value: mint},
          {key: 'Commitment Hash', value: commitmentHash}
        ];
        tables.push(tableData)
        /* if (verbose) {
          const utxo = await iterator.toString();
          this.log("\tStringified UTXO: ", utxo);
        } */
      }
      ux.table(tables[0], {
        utxo: {},
        value: {},
      })
    }
  }

  private logTokenBalance(balances: Balance, inboxBalances: InboxBalance, token: string) {
    
    let balanceObj = this.fetchTokenBalance(balances, token) ?? {};
    let inboxBalanceObj = this.fetchTokenBalance(inboxBalances, token, true) ?? {};
    let tableData = [balanceObj, inboxBalanceObj];
    ux.table(tableData, {
      token: {},
      amount: {},
      balance: {},
      utxos: {},
    })
    this.log('\n');
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
      //this.log(`${JSON.stringify(utxo)}`)
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
    const SOL_DECIMALS = 1_000_000_000;
    return amount / SOL_DECIMALS
  }
}

module.exports = BalanceCommand;
