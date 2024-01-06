import { assert, expect } from "chai";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
import { BN } from "@coral-xyz/anchor";
// Load chai-as-promised support
chai.use(chaiAsPromised);

import { SystemProgram, Keypair as SolanaKeypair } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { it } from "mocha";

import {
  TransactionErrorCode,
  Action,
  TOKEN_REGISTRY,
  Utxo,
  CreateUtxoErrorCode,
  selectInUtxos,
  SelectInUtxosError,
  RelayerErrorCode,
  SelectInUtxosErrorCode,
  Account,
  createRecipientUtxos,
  Provider,
  RELAYER_FEE,
  BN_0,
  BN_1,
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
    utxo1 = new Utxo({
      lightWasm,
      assets: [SystemProgram.programId, tokenCtx.mint],
      amounts: [new BN(1e6), new BN(6 * tokenCtx.decimals.toNumber())],
      index: 0,
      publicKey: utxo1Burner.keypair.publicKey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    utxo2 = new Utxo({
      lightWasm,
      assets: [SystemProgram.programId, tokenCtx.mint],
      amounts: [new BN(1e6), new BN(5 * tokenCtx.decimals.toNumber())],
      index: 0,
      publicKey: utxo2Burner.keypair.publicKey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    utxoSol = new Utxo({
      lightWasm,
      assets: [SystemProgram.programId],
      amounts: [new BN(1e8)],
      index: 1,
      publicKey: utxoSolBurner.keypair.publicKey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
  });

  it("Unshield select spl", async () => {
    const inUtxos: Utxo[] = [utxo1, utxoSol];

    const selectedUtxo = selectInUtxos({
      publicMint: utxo1.assets[1],
      relayerFee: RELAYER_FEE,
      publicAmountSpl: BN_1,
      lightWasm,
      utxos: inUtxos,
      action: Action.UNSHIELD,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });
    Utxo.equal(selectedUtxo[0], utxo1, lightWasm);
  });

  it("Unshield select sol", async () => {
    const inUtxos = [utxoSol, utxo1];

    const selectedUtxo = selectInUtxos({
      utxos: inUtxos,
      relayerFee: RELAYER_FEE,
      publicAmountSol: new BN(1e7),
      lightWasm,
      action: Action.UNSHIELD,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });

    Utxo.equal(selectedUtxo[0], utxoSol, lightWasm);
    assert.equal(selectInUtxos.length, 1);
  });

  it("UNSHIELD select sol & spl", async () => {
    const inUtxos = [utxoSol, utxo1];

    const selectedUtxo = selectInUtxos({
      utxos: inUtxos,
      action: Action.UNSHIELD,
      relayerFee: RELAYER_FEE,
      lightWasm,
      publicMint: utxo1.assets[1],
      publicAmountSol: new BN(1e7),
      publicAmountSpl: BN_1,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });

    Utxo.equal(selectedUtxo[1], utxoSol, lightWasm);
    Utxo.equal(selectedUtxo[0], utxo1, lightWasm);
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
      relayerFee: RELAYER_FEE,
      lightWasm,
      outUtxos,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });

    Utxo.equal(selectedUtxo[1], utxoSol, lightWasm);
    Utxo.equal(selectedUtxo[0], utxo1, lightWasm);
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
      relayerFee: RELAYER_FEE,
      lightWasm,
      outUtxos,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });

    Utxo.equal(selectedUtxo[0], utxoSol, lightWasm);
    Utxo.equal(selectedUtxo[1], utxo1, lightWasm);
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
      relayerFee: RELAYER_FEE,
      lightWasm,
      outUtxos,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });

    Utxo.equal(selectedUtxo[0], utxo1, lightWasm);
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

    Utxo.equal(selectedUtxo[0], utxo1, lightWasm);
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

    Utxo.equal(selectedUtxo[0], utxoSol, lightWasm);
    Utxo.equal(selectedUtxo[1], utxo1, lightWasm);
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

    Utxo.equal(selectedUtxo[0], utxo1, lightWasm);
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
      relayerFee: RELAYER_FEE,
      lightWasm,
      outUtxos,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });

    Utxo.equal(selectedUtxo[0], utxo1, lightWasm);
    Utxo.equal(selectedUtxo[1], utxo2, lightWasm);
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
    account = Account.random(lightWasm);
    utxo1 = new Utxo({
      lightWasm,
      assets: [SystemProgram.programId, tokenCtx.mint],
      amounts: [new BN(1e6), new BN(5 * tokenCtx.decimals.toNumber())],
      index: 0,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      publicKey: account.keypair.publicKey,
    });
    utxo2 = new Utxo({
      lightWasm,
      assets: [SystemProgram.programId, tokenCtx.mint],
      amounts: [new BN(1e6), new BN(5 * tokenCtx.decimals.toNumber())],
      index: 0,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      publicKey: account.keypair.publicKey,
    });
    utxoSol = new Utxo({
      lightWasm,
      assets: [SystemProgram.programId],
      amounts: [new BN(1e8)],
      index: 1,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      publicKey: account.keypair.publicKey,
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
        relayerFee: RELAYER_FEE,
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
        relayerFee: RELAYER_FEE,
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

  it("RELAYER_FEE_UNDEFINED", async () => {
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
        code: RelayerErrorCode.RELAYER_FEE_UNDEFINED,
        functionName: "selectInUtxos",
      });
  });

  it("RELAYER_FEE_UNDEFINED", async () => {
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
        code: RelayerErrorCode.RELAYER_FEE_UNDEFINED,
        functionName: "selectInUtxos",
      });
  });

  it("RELAYER_FEE_DEFINED", async () => {
    const inUtxos = [utxoSol, utxo1];

    expect(() => {
      selectInUtxos({
        utxos: inUtxos,
        action: Action.SHIELD,
        relayerFee: RELAYER_FEE,
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
        code: CreateUtxoErrorCode.RELAYER_FEE_DEFINED,
        functionName: "selectInUtxos",
      });
  });

  it("NO_UTXOS_PROVIDED", async () => {
    expect(() => {
      selectInUtxos({
        action: Action.TRANSFER,
        relayerFee: RELAYER_FEE,
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
        relayerFee: RELAYER_FEE,
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
        relayerFee: RELAYER_FEE,
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
        relayerFee: RELAYER_FEE,
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
        relayerFee: RELAYER_FEE,
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
