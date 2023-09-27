import { assert, expect } from "chai";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
import { BN } from "@coral-xyz/anchor";
// Load chai-as-promised support
chai.use(chaiAsPromised);

import { SystemProgram, Keypair as SolanaKeypair } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { it } from "mocha";

const circomlibjs = require("circomlibjs");

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
  BN_2,
} from "../src";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
let seed32 = bs58.encode(new Uint8Array(32).fill(1));
const numberMaxInUtxos = 2;
const numberMaxOutUtxos = 2;
// TODO: add more tests with different numbers of utxos
// TODO: add a randomized test
describe("Test selectInUtxos Functional", () => {
  let poseidon: any;

  let splAmount,
    token,
    tokenCtx,
    utxo1: Utxo,
    utxo2: Utxo,
    utxoSol: Utxo,
    utxoSolBurner,
    utxo2Burner,
    utxo1Burner;
  let lightProvider: Provider;
  before(async () => {
    lightProvider = await Provider.loadMock();
    poseidon = await circomlibjs.buildPoseidonOpt();
    utxo1Burner = Account.createBurner(poseidon, seed32, BN_0);
    utxo2Burner = Account.createBurner(poseidon, seed32, BN_1);
    utxoSolBurner = Account.createBurner(poseidon, seed32, BN_2);

    splAmount = new BN(3);
    token = "USDC";
    tokenCtx = TOKEN_REGISTRY.get(token);
    if (!tokenCtx) throw new Error("Token not supported!");
    splAmount = splAmount.mul(new BN(tokenCtx.decimals));
    utxo1 = new Utxo({
      poseidon,
      assets: [SystemProgram.programId, tokenCtx.mint],
      amounts: [new BN(1e6), new BN(6 * tokenCtx.decimals.toNumber())],
      index: 0,
      account: utxo1Burner,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    utxo2 = new Utxo({
      poseidon,
      assets: [SystemProgram.programId, tokenCtx.mint],
      amounts: [new BN(1e6), new BN(5 * tokenCtx.decimals.toNumber())],
      index: 1,
      account: utxo2Burner,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    utxoSol = new Utxo({
      poseidon,
      assets: [SystemProgram.programId],
      amounts: [new BN(1e8)],
      index: 2,
      account: utxoSolBurner,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
  });

  it("Unshield select spl", async () => {
    const utxos: Utxo[] = [utxo1, utxoSol];

    let selectedUtxos = selectInUtxos({
      publicMint: utxo1.assets[1],
      relayerFee: RELAYER_FEE,
      publicAmountSpl: BN_1,
      poseidon,
      utxos,
      action: Action.UNSHIELD,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });
    Utxo.equal(poseidon, selectedUtxos[0], utxo1);
  });

  it("Unshield select sol", async () => {
    const utxos = [utxoSol, utxo1];

    let selectedUtxos = selectInUtxos({
      utxos,
      relayerFee: RELAYER_FEE,
      publicAmountSol: new BN(1e7),
      poseidon,
      action: Action.UNSHIELD,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });

    Utxo.equal(poseidon, selectedUtxos[0], utxoSol);
    assert.equal(selectInUtxos.length, 1);
  });

  it("UNSHIELD select sol & spl", async () => {
    const utxos = [utxoSol, utxo1];

    let selectedUtxos = selectInUtxos({
      utxos,
      action: Action.UNSHIELD,
      relayerFee: RELAYER_FEE,
      poseidon,
      publicMint: utxo1.assets[1],
      publicAmountSol: new BN(1e7),
      publicAmountSpl: BN_1,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });

    Utxo.equal(poseidon, selectedUtxos[1], utxoSol);
    Utxo.equal(poseidon, selectedUtxos[0], utxo1);
  });

  it("Transfer select sol & spl", async () => {
    const utxos = [utxoSol, utxo1];
    const outUtxos = createRecipientUtxos({
      recipients: [
        {
          mint: utxo1.assets[1],
          solAmount: new BN(1e7),
          splAmount: BN_1,
          account: new Account({ poseidon }),
        },
      ],
      poseidon,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    let selectedUtxos = selectInUtxos({
      utxos,
      action: Action.TRANSFER,
      relayerFee: RELAYER_FEE,
      poseidon,
      outUtxos,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });

    Utxo.equal(poseidon, selectedUtxos[1], utxoSol);
    Utxo.equal(poseidon, selectedUtxos[0], utxo1);
  });

  it("Transfer select sol", async () => {
    const utxos = [utxoSol, utxo1];
    const outUtxos = createRecipientUtxos({
      recipients: [
        {
          mint: utxo1.assets[1],
          solAmount: new BN(1e7),
          splAmount: BN_0,
          account: new Account({ poseidon }),
        },
      ],
      poseidon,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    let selectedUtxos = selectInUtxos({
      utxos,
      action: Action.TRANSFER,
      relayerFee: RELAYER_FEE,
      poseidon,
      outUtxos,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });

    Utxo.equal(poseidon, selectedUtxos[0], utxoSol);
    Utxo.equal(poseidon, selectedUtxos[1], utxo1);
  });

  it("Transfer select spl", async () => {
    const utxos = [utxoSol, utxo1];
    const outUtxos = createRecipientUtxos({
      recipients: [
        {
          mint: utxo1.assets[1],
          solAmount: BN_0,
          splAmount: BN_1,
          account: new Account({ poseidon }),
        },
      ],
      poseidon,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    let selectedUtxos = selectInUtxos({
      utxos,
      action: Action.TRANSFER,
      relayerFee: RELAYER_FEE,
      poseidon,
      outUtxos,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });

    Utxo.equal(poseidon, selectedUtxos[0], utxo1);
  });

  it("Shield select sol & spl", async () => {
    const utxos = [utxoSol, utxo1];

    let selectedUtxos = selectInUtxos({
      utxos,
      action: Action.SHIELD,
      publicMint: utxo1.assets[1],
      publicAmountSol: new BN(1e7),
      poseidon,
      publicAmountSpl: BN_1,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });

    Utxo.equal(poseidon, selectedUtxos[0], utxo1);
  });

  it("Shield select sol", async () => {
    const utxos = [utxoSol, utxo1];

    let selectedUtxos = selectInUtxos({
      utxos,
      action: Action.SHIELD,
      poseidon,
      publicAmountSol: new BN(1e7),
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });

    Utxo.equal(poseidon, selectedUtxos[0], utxoSol);
    Utxo.equal(poseidon, selectedUtxos[1], utxo1);
  });

  it("Shield select spl", async () => {
    const utxos = [utxoSol, utxo1];

    let selectedUtxos = selectInUtxos({
      utxos,
      action: Action.SHIELD,
      publicMint: utxo1.assets[1],
      poseidon,
      publicAmountSpl: BN_1,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });

    Utxo.equal(poseidon, selectedUtxos[0], utxo1);
    assert.equal(selectedUtxos.length, 1);
  });

  it("3 utxos spl & sol", async () => {
    const utxos = [utxoSol, utxo1, utxo2];
    const outUtxos = createRecipientUtxos({
      recipients: [
        {
          mint: utxo1.assets[1],
          solAmount: utxo2.amounts[0],
          splAmount: utxo2.amounts[1].add(utxo1.amounts[1]),
          account: new Account({ poseidon }),
        },
      ],
      poseidon,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    let selectedUtxos = selectInUtxos({
      utxos,
      action: Action.TRANSFER,
      relayerFee: RELAYER_FEE,
      poseidon,
      outUtxos,
      numberMaxInUtxos,
      numberMaxOutUtxos,
    });

    Utxo.equal(poseidon, selectedUtxos[0], utxo1);
    Utxo.equal(poseidon, selectedUtxos[1], utxo2);
  });
});

describe("Test selectInUtxos Errors", () => {
  let poseidon: any;
  let splAmount,
    token,
    tokenCtx,
    utxo1: Utxo,
    utxo2: Utxo,
    utxoSol: Utxo,
    lightProvider: Provider;

  before(async () => {
    lightProvider = await Provider.loadMock();
    poseidon = await circomlibjs.buildPoseidonOpt();
    splAmount = new BN(3);
    token = "USDC";
    tokenCtx = TOKEN_REGISTRY.get(token);
    if (!tokenCtx) throw new Error("Token not supported!");
    splAmount = splAmount.mul(new BN(tokenCtx.decimals));
    utxo1 = new Utxo({
      poseidon,
      assets: [SystemProgram.programId, tokenCtx.mint],
      amounts: [new BN(1e6), new BN(5 * tokenCtx.decimals.toNumber())],
      index: 0,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    utxo2 = new Utxo({
      poseidon,
      assets: [SystemProgram.programId, tokenCtx.mint],
      amounts: [new BN(1e6), new BN(5 * tokenCtx.decimals.toNumber())],
      index: 0,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    utxoSol = new Utxo({
      poseidon,
      assets: [SystemProgram.programId],
      amounts: [new BN(1e8)],
      index: 1,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
  });

  it("NO_PUBLIC_AMOUNTS_PROVIDED", async () => {
    const utxos = [utxoSol, utxo1];
    const outUtxos = createRecipientUtxos({
      recipients: [
        {
          mint: utxo1.assets[1],
          solAmount: new BN(1e7),
          splAmount: BN_1,
          account: new Account({ poseidon }),
        },
      ],
      poseidon,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    expect(() => {
      selectInUtxos({
        utxos,
        action: Action.UNSHIELD,
        poseidon,
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
    const utxos = [utxoSol, utxo1];

    expect(() => {
      selectInUtxos({
        utxos,
        action: Action.UNSHIELD,
        relayerFee: RELAYER_FEE,
        poseidon,
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
    const utxos = [utxoSol, utxo1];

    expect(() => {
      selectInUtxos({
        utxos,
        action: Action.UNSHIELD,
        relayerFee: RELAYER_FEE,
        poseidon,
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
    const utxos = [utxoSol, utxo1];

    expect(() => {
      selectInUtxos({
        utxos,
        action: Action.UNSHIELD,
        poseidon,
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
    const utxos = [utxoSol, utxo1];

    expect(() => {
      selectInUtxos({
        utxos,
        action: Action.TRANSFER,
        poseidon,
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
    const utxos = [utxoSol, utxo1];

    expect(() => {
      selectInUtxos({
        utxos,
        action: Action.SHIELD,
        relayerFee: RELAYER_FEE,
        poseidon,
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
        poseidon,
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
          account: new Account({ poseidon }),
        },
        {
          mint,
          solAmount: new BN(1e7),
          splAmount: BN_1,
          account: new Account({ poseidon }),
        },
      ],
      poseidon,
      assetLookupTable: [
        ...lightProvider.lookUpTables.assetLookupTable,
        ...[mint.toBase58()],
      ],
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    expect(() => {
      selectInUtxos({
        utxos: inUtxos,
        action: Action.TRANSFER,
        relayerFee: RELAYER_FEE,
        poseidon,
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
    const utxos = [utxoSol, utxo1];
    const outUtxos = createRecipientUtxos({
      recipients: [
        {
          mint: utxo1.assets[1],
          solAmount: new BN(2e10),
          splAmount: BN_1,
          account: new Account({ poseidon }),
        },
      ],
      poseidon,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    expect(() => {
      selectInUtxos({
        utxos,
        action: Action.TRANSFER,
        relayerFee: RELAYER_FEE,
        poseidon,
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
    const utxos = [utxoSol, utxo1];
    const outUtxos = createRecipientUtxos({
      recipients: [
        {
          mint: utxo1.assets[1],
          solAmount: BN_0,
          splAmount: new BN(1e10),
          account: new Account({ poseidon }),
        },
      ],
      poseidon,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    expect(() => {
      selectInUtxos({
        utxos,
        action: Action.TRANSFER,
        relayerFee: RELAYER_FEE,
        poseidon,
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
    const utxos = [utxoSol, utxo1, utxo2];
    const outUtxos = createRecipientUtxos({
      recipients: [
        {
          mint: utxo1.assets[1],
          solAmount: utxo2.amounts[0].add(utxo1.amounts[0]),
          splAmount: utxo2.amounts[1].add(utxo1.amounts[1]),
          account: new Account({ poseidon }),
        },
      ],
      poseidon,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    expect(() => {
      selectInUtxos({
        utxos,
        action: Action.TRANSFER,
        relayerFee: RELAYER_FEE,
        poseidon,
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
