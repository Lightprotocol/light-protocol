//@ts-check
import { Command, Flags, ux } from "@oclif/core";
import {
  User,
  Balance,
  InboxBalance,
  TOKEN_REGISTRY,
  UserError,
  UserErrorCode,
} from "@lightprotocol/zk.js";
import { CustomLoader, getUser, standardFlags } from "../../utils";
import { PublicKey } from "@solana/web3.js";

class BalanceCommand extends Command {
  static description =
    "Show user main and inbox balances as well as respective utxos";

  static flags = {
    ...standardFlags,
    token: Flags.string({
      char: "t",
      description: "The SPL token symbol.",
      default: undefined,
      exclusive: ["inbox"],
      parse: async (token) => token.toUpperCase(),
    }),
    inbox: Flags.boolean({
      char: "i",
      description: "Show user inbox balances.",
      default: false,
    }),
    utxos: Flags.boolean({
      char: "u",
      description: "Show balance utxos.",
      default: false,
    }),
    latest: Flags.boolean({
      char: "l",
      description: "Retrieve the latest balance, inbox balance, or utxos.",
      hidden: true,
      default: true,
    }),
    "all-utxos": Flags.boolean({
      char: "a",
      description:
        "Show main & inbox balances as well as all utxos including main balance utxos.",
      exclusive: ["utxos", "inbox", "token"],
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
    const token = flags.token ?? "none";
    const allUtxos = flags["all-utxos"];

    const loader = new CustomLoader("Retrieving balance...");
    loader.start();

    try {
      const user: User = await getUser({
        skipFetchBalance: flags["skipFetchBalance"],
        localTestRelayer: flags["localTestRelayer"],
      });
      const balances = user.balance;
      const inboxBalances = await user.getUtxoInbox(latest);

      /// Main balance command: no token input
      if (token === "none") {
        this.logBalances(balances);
        if (inbox) {
          this.logBalances(inboxBalances, true);
          if (utxos) {
            this.logUtxos(inboxBalances, true, "all");
          }
        }
        if (allUtxos) {
          this.logBalances(inboxBalances, true);
          this.logUtxos(balances, false, "all");
          this.logUtxos(inboxBalances, true, "all");
        }
      }
      /// Balance for when a token is specified
      else {
        this.log("\n");
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
            this.logUtxos(balances, false, "SOL");
            this.logUtxos(inboxBalances, true, "SOL");
          } else {
            const BLUE = "\x1b[34m%s\x1b[0m";
            this.log(BLUE, `\n--- Main ${token} utxos ---\n`);
            this.logTokenUtxos(balances, tokenCtx!.mint);
            this.log(BLUE, `\n--- Inbox ${token} utxos ---\n`);
            this.logTokenUtxos(inboxBalances, tokenCtx!.mint);
          }
        }
      }
      loader.stop(false);
    } catch (error) {
      this.error(`Failed to show balance!\n${error}`);
    }
  }

  private logBalances(balances: Balance | InboxBalance, _inbox = false) {
    const PURPLE = "\x1b[35m%s\x1b[0m";
    if (_inbox) this.log(PURPLE, "\n--- Inbox Balances ---\n");
    else this.log(PURPLE, "\n--- Main Balances ---\n");
    type TableData = {
        token: string;
        balance: string;
        utxos: number;
    }

    const tableData: TableData[] = [];
    for (const tokenBalance of balances.tokenBalances) {
      const _token = tokenBalance[1].tokenData.symbol;
      const balance =
        _token === "SOL"
          ? tokenBalance[1].totalBalanceSol.toString()
          : tokenBalance[1].totalBalanceSpl.toString();
      const utxoNumber = tokenBalance[1].utxos.size;

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
    });
  }

  private logUtxos(
    balances: Balance | InboxBalance,
    _inbox = false,
    filter?: "SOL" | "all",
  ) {
    const BLUE = "\x1b[34m%s\x1b[0m";
    let logHeader = "";
    switch (filter) {
      case "all":
        if (_inbox) logHeader = `\n--- All Inbox Balance utxos ---\n`;
        else logHeader = `\n--- All Main Balance utxos ---\n`;
        break;
      case "SOL":
        if (_inbox) this.log(BLUE, `\n--- Inbox SOL utxos ---\n`);
        else this.log(BLUE, `\n--- Main SOL utxos ---\n`);
        break;
    }

    for (const tokenBalance of balances.tokenBalances) {
      let i = 0;
      const tableData = [];
      const _token = tokenBalance[1].tokenData.symbol;
      if (_token === "SOL") {
        for (const iterator of tokenBalance[1].utxos.values()!) {
          i++;
          const amountSol = iterator.amounts[0].toString();

          const symbol = tokenBalance[1].tokenData.symbol;
          const commitmentHash = iterator._commitment;

          tableData.push(
            { prop: "Utxo No", value: i },
            { prop: "Token", value: `\x1b[32m${symbol}\x1b[0m` },
            { prop: "Amount LAMPORTS", value: amountSol },
            { prop: "Commitment Hash", value: commitmentHash },
          );
        }
        if (tableData.length === 0) {
          if (!_inbox) this.log("\nThere are no Main Balance utxos to show!");
          else this.log("\nThere are no Inbox Balance utxos to show!");
        } else {
          this.log(BLUE, logHeader);
          ux.table(tableData, {
            prop: { header: "" },
            value: { header: "" },
          });
        }
        if (filter === "SOL") break;
      } else {
        let i = 0;
        const tableData = [];

        for (const iterator of tokenBalance[1].utxos.values()!) {
          i++;
          const amountSpl = iterator.amounts[1].toString();

          const amountSol = iterator.amounts[0].toString();

          const symbol = tokenBalance[1].tokenData.symbol;
          const mint = tokenBalance[1].tokenData.mint.toString();
          const commitmentHash = iterator._commitment;

          tableData.push(
            { prop: "Utxo No", value: i },
            { prop: "Token", value: `\x1b[32m${symbol}\x1b[0m` },
            { prop: "Amount SPL", value: amountSpl },
            { prop: "Amount LAMPORTS", value: amountSol },
            { prop: "Mint", value: mint },
            { prop: "Commitment Hash", value: commitmentHash },
          );
        }
        ux.table(tableData, {
          prop: { header: "" },
          value: { header: "" },
        });
      }
    }
  }

  private logTokenBalance(
    balances: Balance,
    inboxBalances: InboxBalance,
    token: string,
  ) {
    function fetchTokenBalance(
      balances: Balance | InboxBalance,
      token: string,
      inbox = false,
    ) {
      for (const tokenBalance of balances.tokenBalances) {
        const _token = tokenBalance[1].tokenData.symbol;
        if (token === _token) {
          const balance =
            token === "SOL"
              ? tokenBalance[1].totalBalanceSol.toString()
              : tokenBalance[1].totalBalanceSpl.toString();
          const utxoNumber = tokenBalance[1].utxos.size;
          const type = inbox ? "inbox" : "main";
          return {
            token: _token,
            amount: balance,
            balance: type,
            utxos: utxoNumber,
          };
        }
      }
    }

    const balanceObj = fetchTokenBalance(balances, token) ?? {
      token: token,
      amount: 0,
      balance: "main",
      utxos: 0,
    };
    const inboxBalanceObj = fetchTokenBalance(inboxBalances, token, true) ?? {
      token: token,
      amount: 0,
      balance: "inbox",
      utxos: 0,
    };

    const tableData = [balanceObj, inboxBalanceObj];
    ux.table(tableData, {
      token: {},
      amount: {},
      balance: {},
      utxos: {},
    });
  }

  // TODO: support verbose flag
  private logTokenUtxos(balance: Balance, token: PublicKey) {
    const tokenBalance = balance.tokenBalances.get(token.toString());
    if (tokenBalance && tokenBalance?.tokenData.symbol !== "SOL") {
      let i = 0;
      const tableData = [];
      for (const iterator of tokenBalance.utxos.values()!) {
        i++;
        const amountSpl = iterator.amounts[1].toString();
        const amountSol = iterator.amounts[0].toString();
        const symbol = tokenBalance.tokenData.symbol;
        const mint = tokenBalance.tokenData.mint.toString();
        const commitmentHash = iterator._commitment;
        tableData.push(
          { prop: "Utxo No", value: i },
          { prop: "Token", value: symbol },
          { prop: "Amount LAMPORTS", value: amountSol },
          { prop: "Amount SPL", value: amountSpl },
          { prop: "Mint", value: mint },
          { prop: "Commitment Hash", value: commitmentHash },
        );
      }
      ux.table(tableData, {
        prop: { header: "" },
        value: { header: "" },
      });
    }
  }
}

export default BalanceCommand;
