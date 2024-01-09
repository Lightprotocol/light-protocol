import { assert, expect } from "chai";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
import { BN } from "@coral-xyz/anchor";
// Load chai-as-promised support
chai.use(chaiAsPromised);

import { SystemProgram, Keypair as SolanaKeypair } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { it } from "mocha";
import { compareUtxos } from "./test-utils/compareUtxos";
import {
  TransactionErrorCode,
  Action,
  TOKEN_REGISTRY,
  Utxo,
  CreateUtxoErrorCode,
  selectInUtxos,
  SelectInUtxosError,
  RpcErrorCode,
  SelectInUtxosErrorCode,
  Account,
  createRecipientUtxos,
  Provider,
  RPC_FEE,
  BN_0,
  BN_1,
  createTestInUtxo,
} from "../src";
import { WasmFactory, LightWasm } from "@lightprotocol/account.rs";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

const seed32 = bs58.encode(new Uint8Array(32).fill(1));
const numberMaxInUtxos = 2;
const numberMaxOutUtxos = 2;
// TODO: add more tests with different numbers of utxos
// TODO: add a randomized test
describe("Test selectInUtxos Functional", () => {
  let lightWasm: LightWasm;

  let splAmount,
    token,
    tokenCtx,
    utxo1: Utxo,
    utxo2: Utxo,
    utxoSol: Utxo,
    utxoSolBurner: Account,
    utxo2Burner: Account,
    utxo1Burner: Account;
  let lightProvider: Provider;
  before(async () => {
    lightProvider = await Provider.loadMock();

    lightWasm = await WasmFactory.getInstance();
    utxo1Burner = Account.createFromSeed(lightWasm, seed32);
    utxo2Burner = Account.createBurner(lightWasm, seed32, new anchor.BN("0"));
    utxoSolBurner = Account.createBurner(lightWasm, seed32, new anchor.BN("1"));

    splAmount = new BN(3);
    token = "USDC";
    tokenCtx = TOKEN_REGISTRY.get(token);
    if (!tokenCtx) throw new Error("Token not supported!");
    splAmount = splAmount.mul(new BN(tokenCtx.decimals));
    utxo1 = createTestInUtxo({
      lightWasm,
      assets: [SystemProgram.programId, tokenCtx.mint],
      amounts: [new BN(1e6), new BN(6 * tokenCtx.decimals.toNumber())],
      account: utxo1Burner,
    });
    utxo2 = createTestInUtxo({
      lightWasm,
      assets: [SystemProgram.programId, tokenCtx.mint],
      amounts: [new BN(1e6), new BN(5 * tokenCtx.decimals.toNumber())],
      account: utxo2Burner,
    });
    utxoSol = createTestInUtxo({
      lightWasm,
      assets: [SystemProgram.programId],
      amounts: [new BN(1e8)],
      account: utxoSolBurner,
    });
  });

  it("Unshield select spl", async () => {
    const inUtxos: Utxo[] = [utxo1, utxoSol];

    const selectedUtxo = selectInUtxos({
      publicMint: utxo1.assets[1],
      rpcFee: RPC_FEE,
      publicAmountSpl: BN_1,
      lightWasm,
      utxos: inUtxos,
      action: Action.UNSHIELD,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });
    compareUtxos(selectedUtxo[0], utxo1);
  });

  it("Unshield select sol", async () => {
    const inUtxos = [utxoSol, utxo1];

    const selectedUtxo = selectInUtxos({
      utxos: inUtxos,
      rpcFee: RPC_FEE,
      publicAmountSol: new BN(1e7),
      lightWasm,
      action: Action.UNSHIELD,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });

    compareUtxos(selectedUtxo[0], utxoSol);
    assert.equal(selectInUtxos.length, 1);
  });

  it("UNSHIELD select sol & spl", async () => {
    const inUtxos = [utxoSol, utxo1];

    const selectedUtxo = selectInUtxos({
      utxos: inUtxos,
      action: Action.UNSHIELD,
      rpcFee: RPC_FEE,
      lightWasm,
      publicMint: utxo1.assets[1],
      publicAmountSol: new BN(1e7),
      publicAmountSpl: BN_1,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });

    compareUtxos(selectedUtxo[1], utxoSol);
    compareUtxos(selectedUtxo[0], utxo1);
  });

  it("Transfer select sol & spl", async () => {
    const inUtxos = [utxoSol, utxo1];
    const outUtxos = createRecipientUtxos({
      recipients: [
        {
          mint: utxo1.assets[1],
          solAmount: new BN(1e7),
          splAmount: BN_1,
          account: Account.random(lightWasm),
        },
      ],
      lightWasm,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });

    const selectedUtxo = selectInUtxos({
      utxos: inUtxos,
      action: Action.TRANSFER,
      rpcFee: RPC_FEE,
      lightWasm,
      outUtxos,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });

    compareUtxos(selectedUtxo[1], utxoSol);
    compareUtxos(selectedUtxo[0], utxo1);
  });

  it("Transfer select sol", async () => {
    const inUtxos = [utxoSol, utxo1];
    const outUtxos = createRecipientUtxos({
      recipients: [
        {
          mint: utxo1.assets[1],
          solAmount: new BN(1e7),
          splAmount: BN_0,
          account: Account.random(lightWasm),
        },
      ],
      lightWasm,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    const selectedUtxo = selectInUtxos({
      utxos: inUtxos,
      action: Action.TRANSFER,
      rpcFee: RPC_FEE,
      lightWasm,
      outUtxos,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });

    compareUtxos(selectedUtxo[0], utxoSol);
    compareUtxos(selectedUtxo[1], utxo1);
  });

  it("Transfer select spl", async () => {
    const inUtxos = [utxoSol, utxo1];
    const outUtxos = createRecipientUtxos({
      recipients: [
        {
          mint: utxo1.assets[1],
          solAmount: BN_0,
          splAmount: BN_1,
          account: Account.random(lightWasm),
        },
      ],
      lightWasm,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });

    const selectedUtxo = selectInUtxos({
      utxos: inUtxos,
      action: Action.TRANSFER,
      rpcFee: RPC_FEE,
      lightWasm,
      outUtxos,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });

    compareUtxos(selectedUtxo[0], utxo1);
  });

  it("Shield select sol & spl", async () => {
    const inUtxos = [utxoSol, utxo1];

    const selectedUtxo = selectInUtxos({
      utxos: inUtxos,
      action: Action.SHIELD,
      publicMint: utxo1.assets[1],
      publicAmountSol: new BN(1e7),
      lightWasm,
      publicAmountSpl: BN_1,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });

    compareUtxos(selectedUtxo[0], utxo1);
  });

  it("Shield select sol", async () => {
    const inUtxos = [utxoSol, utxo1];

    const selectedUtxo = selectInUtxos({
      utxos: inUtxos,
      action: Action.SHIELD,
      lightWasm,
      publicAmountSol: new BN(1e7),
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });

    compareUtxos(selectedUtxo[0], utxoSol);
    compareUtxos(selectedUtxo[1], utxo1);
  });

  it("Shield select spl", async () => {
    const inUtxos = [utxoSol, utxo1];

    const selectedUtxo = selectInUtxos({
      utxos: inUtxos,
      action: Action.SHIELD,
      publicMint: utxo1.assets[1],
      lightWasm,
      publicAmountSpl: BN_1,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });

    compareUtxos(selectedUtxo[0], utxo1);
    assert.equal(selectedUtxo.length, 1);
  });

  it("3 utxos spl & sol", async () => {
    const inUtxos = [utxoSol, utxo1, utxo2];
    const outUtxos = createRecipientUtxos({
      recipients: [
        {
          mint: utxo1.assets[1],
          solAmount: utxo2.amounts[0],
          splAmount: utxo2.amounts[1].add(utxo1.amounts[1]),
          account: Account.random(lightWasm),
        },
      ],
      lightWasm,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });

    const selectedUtxo = selectInUtxos({
      utxos: inUtxos,
      action: Action.TRANSFER,
      rpcFee: RPC_FEE,
      lightWasm,
      outUtxos,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });

    compareUtxos(selectedUtxo[0], utxo1);
    compareUtxos(selectedUtxo[1], utxo2);
  });
});

describe("Test selectInUtxos Errors", () => {
  let lightWasm: LightWasm;
  let splAmount,
    token,
    tokenCtx,
    utxo1: Utxo,
    utxo2: Utxo,
    utxoSol: Utxo,
    lightProvider: Provider,
    account: Account;

  before(async () => {
    lightProvider = await Provider.loadMock();
    lightWasm = await WasmFactory.getInstance();
    splAmount = new BN(3);
    token = "USDC";
    tokenCtx = TOKEN_REGISTRY.get(token);
    if (!tokenCtx) throw new Error("Token not supported!");
    splAmount = splAmount.mul(new BN(tokenCtx.decimals));
    account = Account.createFromSeed(lightWasm, seed32);
    utxo1 = createTestInUtxo({
      lightWasm,
      assets: [SystemProgram.programId, tokenCtx.mint],
      amounts: [new BN(1e6), new BN(6 * tokenCtx.decimals.toNumber())],
      account,
    });
    utxo2 = createTestInUtxo({
      lightWasm,
      assets: [SystemProgram.programId, tokenCtx.mint],
      amounts: [new BN(1e6), new BN(5 * tokenCtx.decimals.toNumber())],
      account,
    });
    utxoSol = createTestInUtxo({
      lightWasm,
      assets: [SystemProgram.programId],
      amounts: [new BN(1e8)],
      account,
    });
  });

  it("NO_PUBLIC_AMOUNTS_PROVIDED", async () => {
    const inUtxos = [utxoSol, utxo1];
    const outUtxos = createRecipientUtxos({
      recipients: [
        {
          mint: utxo1.assets[1],
          solAmount: new BN(1e7),
          splAmount: BN_1,
          account: Account.random(lightWasm),
        },
      ],
      lightWasm,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    expect(() => {
      selectInUtxos({
        utxos: inUtxos,
        action: Action.UNSHIELD,
        lightWasm,
        outUtxos,
        numberMaxInUtxos,
        numberMaxOutUtxos,
      });
    })
      .to.throw(SelectInUtxosError)
      .includes({
        code: CreateUtxoErrorCode.NO_PUBLIC_AMOUNTS_PROVIDED,
        functionName: "selectInUtxos",
      });
  });

  it("NO_PUBLIC_MINT_PROVIDED", async () => {
    const inUtxos = [utxoSol, utxo1];

    expect(() => {
      selectInUtxos({
        utxos: inUtxos,
        action: Action.UNSHIELD,
        rpcFee: RPC_FEE,
        lightWasm,
        publicAmountSol: new BN(1e7),
        publicAmountSpl: BN_1,
        numberMaxInUtxos,
        numberMaxOutUtxos,
      });
    })
      .to.throw(SelectInUtxosError)
      .includes({
        code: CreateUtxoErrorCode.NO_PUBLIC_MINT_PROVIDED,
        functionName: "selectInUtxos",
      });
  });

  it("PUBLIC_SPL_AMOUNT_UNDEFINED", async () => {
    const inUtxos = [utxoSol, utxo1];

    expect(() => {
      selectInUtxos({
        utxos: inUtxos,
        action: Action.UNSHIELD,
        rpcFee: RPC_FEE,
        lightWasm,
        publicMint: utxo1.assets[1],
        publicAmountSol: new BN(1e7),
        numberMaxInUtxos,
        numberMaxOutUtxos,
      });
    })
      .to.throw(SelectInUtxosError)
      .includes({
        code: CreateUtxoErrorCode.PUBLIC_SPL_AMOUNT_UNDEFINED,
        functionName: "selectInUtxos",
      });
  });

  it("RPC_FEE_UNDEFINED", async () => {
    const inUtxos = [utxoSol, utxo1];

    expect(() => {
      selectInUtxos({
        utxos: inUtxos,
        action: Action.UNSHIELD,
        lightWasm,
        publicMint: utxo1.assets[1],
        publicAmountSol: new BN(1e7),
        publicAmountSpl: BN_1,
        numberMaxInUtxos,
        numberMaxOutUtxos,
      });
    })
      .to.throw(SelectInUtxosError)
      .includes({
        code: RpcErrorCode.RPC_FEE_UNDEFINED,
        functionName: "selectInUtxos",
      });
  });

  it("RPC_FEE_UNDEFINED", async () => {
    const inUtxos = [utxoSol, utxo1];

    expect(() => {
      selectInUtxos({
        utxos: inUtxos,
        action: Action.TRANSFER,
        lightWasm,
        publicMint: utxo1.assets[1],
        publicAmountSol: new BN(1e7),
        publicAmountSpl: BN_1,
        numberMaxInUtxos,
        numberMaxOutUtxos,
      });
    })
      .to.throw(SelectInUtxosError)
      .includes({
        code: RpcErrorCode.RPC_FEE_UNDEFINED,
        functionName: "selectInUtxos",
      });
  });

  it("RPC_FEE_DEFINED", async () => {
    const inUtxos = [utxoSol, utxo1];

    expect(() => {
      selectInUtxos({
        utxos: inUtxos,
        action: Action.SHIELD,
        rpcFee: RPC_FEE,
        lightWasm,
        publicMint: utxo1.assets[1],
        publicAmountSol: new BN(1e7),
        publicAmountSpl: BN_1,
        numberMaxInUtxos,
        numberMaxOutUtxos,
      });
    })
      .to.throw(SelectInUtxosError)
      .includes({
        code: CreateUtxoErrorCode.RPC_FEE_DEFINED,
        functionName: "selectInUtxos",
      });
  });

  it("NO_UTXOS_PROVIDED", async () => {
    expect(() => {
      selectInUtxos({
        action: Action.TRANSFER,
        rpcFee: RPC_FEE,
        lightWasm,
        publicMint: utxo1.assets[1],
        publicAmountSol: new BN(1e7),
        publicAmountSpl: BN_1,
        numberMaxInUtxos,
        numberMaxOutUtxos,
      });
    })
      .to.throw(SelectInUtxosError)
      .includes({
        code: TransactionErrorCode.NO_UTXOS_PROVIDED,
        functionName: "selectInUtxos",
      });
  });

  it("INVALID_NUMBER_OF_RECIPIENTS", async () => {
    const mint = SolanaKeypair.generate().publicKey;
    const inUtxos = [utxoSol, utxo1];
    const outUtxos = createRecipientUtxos({
      recipients: [
        {
          mint: utxo1.assets[1],
          solAmount: new BN(1e7),
          splAmount: BN_1,
          account: Account.random(lightWasm),
        },
        {
          mint,
          solAmount: new BN(1e7),
          splAmount: BN_1,
          account: Account.random(lightWasm),
        },
      ],
      lightWasm,
      assetLookupTable: [
        ...lightProvider.lookUpTables.assetLookupTable,
        ...[mint.toBase58()],
      ],
    });
    expect(() => {
      selectInUtxos({
        utxos: inUtxos,
        action: Action.TRANSFER,
        rpcFee: RPC_FEE,
        lightWasm,
        outUtxos,
        numberMaxInUtxos,
        numberMaxOutUtxos,
      });
    })
      .to.throw(SelectInUtxosError)
      .includes({
        code: CreateUtxoErrorCode.INVALID_NUMBER_OF_RECIPIENTS,
        functionName: "selectInUtxos",
      });
  });

  it("FAILED_TO_FIND_UTXO_COMBINATION sol", async () => {
    const inUtxos = [utxoSol, utxo1];
    const outUtxos = createRecipientUtxos({
      recipients: [
        {
          mint: utxo1.assets[1],
          solAmount: new BN(2e10),
          splAmount: BN_1,
          account: Account.random(lightWasm),
        },
      ],
      lightWasm,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    expect(() => {
      selectInUtxos({
        utxos: inUtxos,
        action: Action.TRANSFER,
        rpcFee: RPC_FEE,
        lightWasm,
        outUtxos,
        numberMaxInUtxos,
        numberMaxOutUtxos,
      });
    })
      .to.throw(SelectInUtxosError)
      .includes({
        code: SelectInUtxosErrorCode.FAILED_TO_FIND_UTXO_COMBINATION,
        functionName: "selectInUtxos",
      });
  });

  it("FAILED_TO_FIND_UTXO_COMBINATION spl", async () => {
    const inUtxos = [utxoSol, utxo1];
    const outUtxos = createRecipientUtxos({
      recipients: [
        {
          mint: utxo1.assets[1],
          solAmount: BN_0,
          splAmount: new BN(1e10),
          account: Account.random(lightWasm),
        },
      ],
      lightWasm,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    expect(() => {
      selectInUtxos({
        utxos: inUtxos,
        action: Action.TRANSFER,
        rpcFee: RPC_FEE,
        lightWasm,
        outUtxos,
        numberMaxInUtxos,
        numberMaxOutUtxos,
      });
    })
      .to.throw(SelectInUtxosError)
      .includes({
        code: SelectInUtxosErrorCode.FAILED_TO_FIND_UTXO_COMBINATION,
        functionName: "selectInUtxos",
      });
  });

  it("FAILED_TO_FIND_UTXO_COMBINATION spl & sol", async () => {
    const inUtxos = [utxoSol, utxo1, utxo2];
    const outUtxos = createRecipientUtxos({
      recipients: [
        {
          mint: utxo1.assets[1],
          solAmount: utxo2.amounts[0].add(utxo1.amounts[0]),
          splAmount: utxo2.amounts[1].add(utxo1.amounts[1]),
          account: Account.random(lightWasm),
        },
      ],
      lightWasm,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    expect(() => {
      selectInUtxos({
        utxos: inUtxos,
        action: Action.TRANSFER,
        rpcFee: RPC_FEE,
        lightWasm,
        outUtxos,
        numberMaxInUtxos,
        numberMaxOutUtxos,
      });
    })
      .to.throw(SelectInUtxosError)
      .includes({
        code: SelectInUtxosErrorCode.FAILED_TO_FIND_UTXO_COMBINATION,
        functionName: "selectInUtxos",
      });
  });
});
