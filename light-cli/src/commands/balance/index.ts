import { Command, Flags, ux } from "@oclif/core";
import { BN } from "@coral-xyz/anchor";
import {
  User,
  Balance,
  InboxBalance,
  TOKEN_REGISTRY,
  UserError,
  UserErrorCode,
  convertAndComputeDecimals
} from "@lightprotocol/zk.js";
import { CustomLoader, getUser } from "../../utils/utils";
import { PublicKey } from "@solana/web3.js";

class BalanceCommand extends Command {
  static description =
    "Show user main and inbox balances as well as respective UTXOs";

  static flags = {
    token: Flags.string({
      char: "t",
      description: "The SPL token symbol",
      default: undefined,
      exclusive: ['inbox'],
      parse: async (token) => token.toUpperCase(), 
    }),
    inbox: Flags.boolean({
      char: "i",
      description: "Show user inbox balances",
      default: false,
    }),
    utxos: Flags.boolean({
      char: "u",
      description: "Show balance UTXOs",
      default: false,
    }),
    latest: Flags.boolean({
      char: "l",
      description: "Retrieve the latest balance, inbox balance, or UTXOs",
      hidden: true,
      default: true,
    }),
    'all-utxos': Flags.boolean({
      char: "a",
      description: "Show main & inbox balances as well as all UTXOs including main balance UTXOs",
      exclusive: ['utxos', 'inbox', 'token'],
      default: false,
    }),
  };

  static examples = [
    "$ light balance --inbox",
    "$ light balance --inbox --utxos",
    "$ light balance --token USDC",
    "$ light balance --token SOL --utxos",
    "$ light balance --all-utxos",
  ];

  async run() {
    const { flags } = await this.parse(BalanceCommand);
    const { inbox, utxos, latest } = flags;
    const token = flags.token ?? 'none';
    const allUtxos = flags["all-utxos"];

    const loader = new CustomLoader("Retrieving balance...");
    loader.start();

    try {
      const user: User = await getUser();
      const balances = user.balance;
      const inboxBalances = await user.getUtxoInbox(latest);

      /// Main balance command: no token input
      if (token === "none") {
        this.logBalances(balances);
        if (inbox) {
          this.logBalances(inboxBalances, true);
          if (utxos) {
            this.logUtxos(inboxBalances, true, 'all');
          }
        }
        if (allUtxos) {
          this.logBalances(inboxBalances, true);
          this.logUtxos(balances, false, 'all'); 
          this.logUtxos(inboxBalances, true, 'all'); 
        }
      }
      /// Balance for when a token is specified
      else {
        this.log('\n');
        const tokenCtx = TOKEN_REGISTRY.get(token!);
        if (!tokenCtx)
          throw new UserError(
            UserErrorCode.TOKEN_NOT_FOUND,
            "shield",
            "Token not supported!",
        );
        this.logTokenBalance(balances, inboxBalances, token!);

        if (utxos) {
          if (token === "SOL") {
            this.logUtxos(balances, false, 'sol');
            this.logUtxos(inboxBalances, true, 'sol');
          }
          else {
            const BLUE = "\x1b[34m%s\x1b[0m";
            this.log(BLUE, `\n--- Main ${token} UTXOs ---\n`);
            this.logTokenUtxos(balances, tokenCtx!.mint);
            this.log(BLUE, `\n--- Inbox ${token} UTXOs ---\n`);
            this.logTokenUtxos(inboxBalances, tokenCtx!.mint);
          }
        }
      } 
      loader.stop(false);
    } catch (error) {
      this.error(`Failed to show balance!\n${error}`);
    }
  }

  private logBalances(balances: Balance | InboxBalance, _inbox=false) {
    const PURPLE = "\x1b[35m%s\x1b[0m";
    if (_inbox) this.log(PURPLE, "\n--- Inbox Balances ---\n");
    else this.log(PURPLE, "\n--- Main Balances ---\n");

    let tableData = [];
    for (const tokenBalance of balances.tokenBalances) {
      let _token = tokenBalance[1].tokenData.symbol;
      let decimals = tokenBalance[1].tokenData.decimals;
      let balance: BN = _token === "SOL" 
        ? tokenBalance[1].totalBalanceSol / decimals
        : tokenBalance[1].totalBalanceSpl / decimals;
      let utxoNumber = tokenBalance[1].utxos.size;

      tableData.push({
        token: _token,
        balance: balance,
        utxos: utxoNumber,
      });
    } 

    ux.table(tableData, {
      token: {},
      balance: {},
      utxos: {},
    })
  }

  private logUtxos(balances: Balance | InboxBalance, _inbox=false, filter?: "sol" | "all") {
    const BLUE = "\x1b[34m%s\x1b[0m";
    let logHeader = '';
    switch (filter) {
      case 'all':
        if (_inbox) logHeader = `\n--- All Inbox Balance UTXOs ---\n`;
        else logHeader = `\n--- All Main Balance UTXOs ---\n`;
        break;
      case 'sol':
        if (_inbox) this.log(BLUE, `\n--- Inbox SOL UTXOs ---\n`);
        else this.log(BLUE, `\n--- Main SOL UTXOs ---\n`);
        break;
    }
    
    for (const tokenBalance of balances.tokenBalances) {
      let i=0;
      let tableData = [];
      let _token = tokenBalance[1].tokenData.symbol;
      if (_token === "SOL") {
        for (const iterator of tokenBalance[1]?.utxos.values()!) {
          i++;
          let amountSol = this.convertToSol(iterator.amounts[0]);
          let symbol = tokenBalance[1].tokenData.symbol;
          let commitmentHash = iterator._commitment;

          tableData.push(
            {prop: 'Utxo No', value: i},
            {prop: 'Token', value: `\x1b[32m${symbol}\x1b[0m`},
            {prop: 'Amount SOL', value: amountSol},
            {prop: 'Commitment Hash', value: commitmentHash},
          );
        }
        if (tableData.length === 0) {
          if (!_inbox) this.log('\nThere are no Main Balance UTXOs to show!');
          else this.log('\nThere are no Inbox Balance UTXOs to show!');
        }
        else {
          this.log(BLUE, logHeader);
          ux.table(
            tableData, 
            {
              prop: {header: ''},
              value: {header: ''},
            }
          );
        }
        if (filter === 'sol') break;
      }
      else {
        let i=0;
        let decimals = tokenBalance[1].tokenData.decimals;
        let tableData = [];
        for (const iterator of tokenBalance[1].utxos.values()!) {
          i++;
          let amountSpl = iterator.amounts[1] / decimals;
          let amountSol = this.convertToSol(iterator.amounts[0]);
          let symbol = tokenBalance[1].tokenData.symbol;
          let mint = tokenBalance[1].tokenData.mint.toString();
          let commitmentHash = iterator._commitment;

          tableData.push(
            {prop: 'Utxo No', value: i},
            {prop: 'Token', value: `\x1b[32m${symbol}\x1b[0m`},
            {prop: 'Amount SPL', value: amountSpl},
            {prop: 'Amount SOL', value: amountSol},
            {prop: 'Mint', value: mint},
            {prop: 'Commitment Hash', value: commitmentHash},
          )
        }
        ux.table(
          tableData, 
          {
            prop: {header: ''},
            value: {header: ''},
          }
        )
      }
    }
  }

  private logTokenBalance(balances: Balance, inboxBalances: InboxBalance, token: string) {
    function fetchTokenBalance(balances: Balance | InboxBalance, token: string, inbox=false) {
      for (const tokenBalance of balances.tokenBalances) {
        let _token = tokenBalance[1].tokenData.symbol;
        if (token === _token) {
          let decimals = tokenBalance[1].tokenData.decimals;
          let balance = token === "SOL" 
            ? tokenBalance[1].totalBalanceSol / decimals
            : tokenBalance[1].totalBalanceSpl / decimals;
          let utxoNumber = tokenBalance[1].utxos.size;
          let type = inbox ? 'inbox' : 'main';
          return {
            token: _token,
            amount: balance,
            balance: type,
            utxos: utxoNumber
          }
        }
      }
    }
      
    let balanceObj = fetchTokenBalance(balances, token) ?? {
      token: token, 
      amount: 0, 
      balance: 'main', 
      utxos: 0
    };
    let inboxBalanceObj = fetchTokenBalance(inboxBalances, token, true) ?? {
      token: token, 
      amount: 0, 
      balance: 'inbox', 
      utxos: 0
    };

    let tableData = [balanceObj, inboxBalanceObj];
    ux.table(tableData, {
      token: {},
      amount: {},
      balance: {},
      utxos: {},
    })
  }

  private logTokenUtxos(balance: Balance, token: PublicKey, verbose=false) {
    const tokenBalance = balance.tokenBalances.get(token.toString());
    if (tokenBalance && tokenBalance?.tokenData.symbol !== "SOL") {
      let i=0;
      let decimals = tokenBalance.tokenData.decimals;
      let tableData = [];
      for (const iterator of tokenBalance?.utxos.values()!) {
        i++;
        let amountSpl = iterator.amounts[1] / decimals;
        let amountSol = this.convertToSol(iterator.amounts[0]);
        let symbol = tokenBalance.tokenData.symbol;
        let mint = tokenBalance.tokenData.mint.toString();
        let commitmentHash = iterator._commitment;
        tableData.push(
          {prop: 'Utxo No', value: i},
          {prop: 'Token', value: symbol},
          {prop: 'Amount SOL', value: amountSol},
          {prop: 'Amount SPL', value: amountSpl},
          {prop: 'Mint', value: mint},
          {prop: 'Commitment Hash', value: commitmentHash},
        )
      }
      ux.table(tableData, {
        prop: {header: ''},
        value: {header: ''},
      })
    }
  }

  //TODO: use the SOL converter from the light-zk.js => convertAndComputeDecimals(amount, new BN(1e9)).toNumber())
  private convertToSol(amount: BN): number {
    const SOL_DECIMALS = 1_000_000_000;
    return amount / SOL_DECIMALS
  }
}

export default BalanceCommand;


