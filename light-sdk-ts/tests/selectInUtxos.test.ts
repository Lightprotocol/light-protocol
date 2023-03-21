import { assert, expect } from "chai";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
import { BN } from "@coral-xyz/anchor";
// Load chai-as-promised support
chai.use(chaiAsPromised);
let circomlibjs = require("circomlibjs");
import {
  SystemProgram,
  Keypair as SolanaKeypair,
  PublicKey,
} from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { it } from "mocha";
import { buildPoseidonOpt, buildBabyjub, buildEddsa } from "circomlibjs";
import { Scalar } from "ffjavascript";

import { Account } from "../src/account";
import {
  FEE_ASSET,
  functionalCircuitTest,
  hashAndTruncateToCircuit,
  Provider as LightProvider,
  MINT,
  Transaction,
  UtxoError,
  UtxoErrorCode,
  TransactionError,
  TransactionErrorCode,
  ProviderErrorCode,
  Provider,
  TransactionParameters,
  VerifierZero,
  Action,
  Relayer,
  AccountError,
  AccountErrorCode,
  TransactionParametersErrorCode,
  createOutUtxos,
  strToArr,
  ADMIN_AUTH_KEYPAIR,
  TOKEN_REGISTRY,
  User,
  Utxo,
  CreateUtxoError,
  CreateUtxoErrorCode,
  selectInUtxos,
  SelectInUtxosError,
  RelayerErrorCode,
} from "../src";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
let seed32 = new Uint8Array(32).fill(1).toString();

// TODO: add more tests with different numbers of utxos
// TOOD: add a randomized test
describe("Test selectInUtxos Functional", () => {
  var poseidon, eddsa, babyJub, F, k0: Account, k00: Account, kBurner: Account;
  const userKeypair = ADMIN_AUTH_KEYPAIR; //new SolanaKeypair();
  const mockPublicKey = SolanaKeypair.generate().publicKey;

  var splAmount,
    solAmount,
    token,
    tokenCtx,
    utxo1: Utxo,
    relayerFee,
    utxoSol,
    recipientAccount;
  before(async () => {
    poseidon = await circomlibjs.buildPoseidonOpt();
    eddsa = await buildEddsa();
    babyJub = await buildBabyjub();
    F = babyJub.F;
    k0 = new Account({ poseidon, seed: seed32 });
    k00 = new Account({ poseidon, seed: seed32 });
    kBurner = Account.createBurner(poseidon, seed32, new anchor.BN("0"));
    splAmount = new BN(3);
    solAmount = new BN(1e6);
    token = "USDC";
    tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);
    if (!tokenCtx) throw new Error("Token not supported!");
    splAmount = splAmount.mul(new BN(tokenCtx.decimals));
    utxo1 = new Utxo({
      poseidon,
      assets: [SystemProgram.programId, tokenCtx.tokenAccount],
      amounts: [new BN(1e6), new BN(5 * tokenCtx.decimals)],
      index: 0,
    });
    utxoSol = new Utxo({
      poseidon,
      assets: [SystemProgram.programId],
      amounts: [new BN(1e8)],
      index: 1,
    });
    relayerFee = new BN(1000);

    const shieldedRecipient =
      "19a20668193c0143dd96983ef457404280741339b95695caddd0ad7919f2d434";
    const encryptionPublicKey =
      "LPx24bc92eecaf5e3904bc1f4f731a2b1e0a28adf445e800c4cff112eb7a3f5350b";

    const recipient: Uint8Array = strToArr(shieldedRecipient);
    const recipientEncryptionPublicKey: Uint8Array =
      strToArr(encryptionPublicKey);
    recipientAccount = Account.fromPubkey(
      recipient,
      recipientEncryptionPublicKey,
      poseidon,
    );
  });

  it("Unshield select spl", async () => {
    const inUtxos: Utxo[] = [utxo1, utxoSol];

    let selectedUtxo = selectInUtxos({
      publicMint: utxo1.assets[1],
      relayerFee: new BN(1000),
      publicSplAmount: new BN(1),
      utxos: inUtxos,
      action: Action.UNSHIELD,
    });
    Utxo.equal(selectedUtxo[0], utxo1);
  });

  it("Unshield select sol", async () => {
    const inUtxos = [utxoSol, utxo1];

    let selectedUtxo = selectInUtxos({
      utxos: inUtxos,
      relayerFee: new BN(1000),
      publicSolAmount: new BN(1e7),
      action: Action.UNSHIELD,
    });

    Utxo.equal(selectedUtxo[0], utxoSol);
    Utxo.equal(selectedUtxo[1], utxo1);
  });

  it("UNSHIELD select sol & spl", async () => {
    const inUtxos = [utxoSol, utxo1];

    let selectedUtxo = selectInUtxos({
      utxos: inUtxos,
      action: Action.UNSHIELD,
      relayerFee: new BN(1000),
      publicMint: utxo1.assets[1],
      publicSolAmount: new BN(1e7),
      publicSplAmount: new BN(1),
    });

    Utxo.equal(selectedUtxo[1], utxoSol);
    Utxo.equal(selectedUtxo[0], utxo1);
  });

  it("Transfer select sol & spl", async () => {
    const inUtxos = [utxoSol, utxo1];

    let selectedUtxo = selectInUtxos({
      utxos: inUtxos,
      action: Action.TRANSFER,
      relayerFee: new BN(1000),
      recipients: [
        {
          mint: utxo1.assets[1],
          solAmount: new BN(1e7),
          splAmount: new BN(1),
          account: new Account({ poseidon }),
        },
      ],
    });

    Utxo.equal(selectedUtxo[1], utxoSol);
    Utxo.equal(selectedUtxo[0], utxo1);
  });

  it("Transfer select sol", async () => {
    const inUtxos = [utxoSol, utxo1];

    let selectedUtxo = selectInUtxos({
      utxos: inUtxos,
      action: Action.TRANSFER,
      relayerFee: new BN(1000),
      recipients: [
        {
          mint: utxo1.assets[1],
          solAmount: new BN(1e7),
          splAmount: new BN(0),
          account: new Account({ poseidon }),
        },
      ],
    });

    Utxo.equal(selectedUtxo[0], utxoSol);
    Utxo.equal(selectedUtxo[1], utxo1);
  });

  it("Transfer select spl", async () => {
    const inUtxos = [utxoSol, utxo1];

    let selectedUtxo = selectInUtxos({
      utxos: inUtxos,
      action: Action.TRANSFER,
      relayerFee: new BN(1000),
      recipients: [
        {
          mint: utxo1.assets[1],
          solAmount: new BN(0),
          splAmount: new BN(1),
          account: new Account({ poseidon }),
        },
      ],
    });

    Utxo.equal(selectedUtxo[0], utxo1);
  });

  it("Shield select sol & spl", async () => {
    const inUtxos = [utxoSol, utxo1];

    let selectedUtxo = selectInUtxos({
      utxos: inUtxos,
      action: Action.SHIELD,
      publicMint: utxo1.assets[1],
      publicSolAmount: new BN(1e7),
      publicSplAmount: new BN(1),
    });

    Utxo.equal(selectedUtxo[0], utxo1);
  });

  it("Shield select sol", async () => {
    const inUtxos = [utxoSol, utxo1];

    let selectedUtxo = selectInUtxos({
      utxos: inUtxos,
      action: Action.SHIELD,
      publicSolAmount: new BN(1e7),
    });

    Utxo.equal(selectedUtxo[0], utxoSol);
    Utxo.equal(selectedUtxo[1], utxo1);
  });

  it("Shield select spl", async () => {
    const inUtxos = [utxoSol, utxo1];

    let selectedUtxo = selectInUtxos({
      utxos: inUtxos,
      action: Action.SHIELD,
      publicMint: utxo1.assets[1],
      publicSplAmount: new BN(1),
    });

    Utxo.equal(selectedUtxo[0], utxo1);
  });
});

describe("Test selectInUtxos Errors", () => {
  var poseidon, eddsa, babyJub, F, k0: Account, k00: Account, kBurner: Account;
  const userKeypair = ADMIN_AUTH_KEYPAIR; //new SolanaKeypair();
  const mockPublicKey = SolanaKeypair.generate().publicKey;

  var splAmount,
    solAmount,
    token,
    tokenCtx,
    utxo1: Utxo,
    relayerFee,
    utxoSol,
    recipientAccount;
  before(async () => {
    poseidon = await circomlibjs.buildPoseidonOpt();
    eddsa = await buildEddsa();
    babyJub = await buildBabyjub();
    F = babyJub.F;
    k0 = new Account({ poseidon, seed: seed32 });
    k00 = new Account({ poseidon, seed: seed32 });
    kBurner = Account.createBurner(poseidon, seed32, new anchor.BN("0"));
    splAmount = new BN(3);
    solAmount = new BN(1e6);
    token = "USDC";
    tokenCtx = TOKEN_REGISTRY.find((t) => t.symbol === token);
    if (!tokenCtx) throw new Error("Token not supported!");
    splAmount = splAmount.mul(new BN(tokenCtx.decimals));
    utxo1 = new Utxo({
      poseidon,
      assets: [SystemProgram.programId, tokenCtx.tokenAccount],
      amounts: [new BN(1e6), new BN(5 * tokenCtx.decimals)],
      index: 0,
    });
    utxoSol = new Utxo({
      poseidon,
      assets: [SystemProgram.programId],
      amounts: [new BN(1e8)],
      index: 1,
    });
    relayerFee = new BN(1000);

    const shieldedRecipient =
      "19a20668193c0143dd96983ef457404280741339b95695caddd0ad7919f2d434";
    const encryptionPublicKey =
      "LPx24bc92eecaf5e3904bc1f4f731a2b1e0a28adf445e800c4cff112eb7a3f5350b";

    const recipient: Uint8Array = strToArr(shieldedRecipient);
    const recipientEncryptionPublicKey: Uint8Array =
      strToArr(encryptionPublicKey);
    recipientAccount = Account.fromPubkey(
      recipient,
      recipientEncryptionPublicKey,
      poseidon,
    );
  });

  it("NO_PUBLIC_AMOUNTS_PROVIDED", async () => {
    const inUtxos = [utxoSol, utxo1];

    expect(() => {
      selectInUtxos({
        utxos: inUtxos,
        action: Action.UNSHIELD,
        recipients: [
          {
            mint: utxo1.assets[1],
            solAmount: new BN(1e7),
            splAmount: new BN(1),
            account: new Account({ poseidon }),
          },
        ],
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
        relayerFee: new BN(1000),
        // publicMint: utxo1.assets[1],
        publicSolAmount: new BN(1e7),
        publicSplAmount: new BN(1),
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
        relayerFee: new BN(1000),
        publicMint: utxo1.assets[1],
        publicSolAmount: new BN(1e7),
        // publicSplAmount: new BN(1),
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
        // relayerFee: new BN(1000),
        publicMint: utxo1.assets[1],
        publicSolAmount: new BN(1e7),
        publicSplAmount: new BN(1),
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
        // relayerFee: new BN(1000),
        publicMint: utxo1.assets[1],
        publicSolAmount: new BN(1e7),
        publicSplAmount: new BN(1),
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
        relayerFee: new BN(1000),
        publicMint: utxo1.assets[1],
        publicSolAmount: new BN(1e7),
        publicSplAmount: new BN(1),
      });
    })
      .to.throw(SelectInUtxosError)
      .includes({
        code: CreateUtxoErrorCode.RELAYER_FEE_DEFINED,
        functionName: "selectInUtxos",
      });
  });

  it("NO_UTXOS_PROVIDED", async () => {
    const inUtxos = [utxoSol, utxo1];

    expect(() => {
      selectInUtxos({
        // utxos: inUtxos,
        action: Action.TRANSFER,
        relayerFee: new BN(1000),
        publicMint: utxo1.assets[1],
        publicSolAmount: new BN(1e7),
        publicSplAmount: new BN(1),
      });
    })
      .to.throw(SelectInUtxosError)
      .includes({
        code: TransactionErrorCode.NO_UTXOS_PROVIDED,
        functionName: "selectInUtxos",
      });
  });

  it("INVALID_NUMER_OF_RECIPIENTS", async () => {
    const inUtxos = [utxoSol, utxo1];

    expect(() => {
      selectInUtxos({
        utxos: inUtxos,
        action: Action.TRANSFER,
        relayerFee: new BN(1000),
        recipients: [
          {
            mint: utxo1.assets[1],
            solAmount: new BN(1e7),
            splAmount: new BN(1),
            account: new Account({ poseidon }),
          },
          {
            mint: SolanaKeypair.generate().publicKey,
            solAmount: new BN(1e7),
            splAmount: new BN(1),
            account: new Account({ poseidon }),
          },
        ],
      });
    })
      .to.throw(SelectInUtxosError)
      .includes({
        code: CreateUtxoErrorCode.INVALID_NUMER_OF_RECIPIENTS,
        functionName: "selectInUtxos",
      });
  });
});
