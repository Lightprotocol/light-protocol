import { assert } from "chai";
import { SystemProgram } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { it } from "mocha";

const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
// Load chai-as-promised support
chai.use(chaiAsPromised);

import {
  FEE_ASSET,
  Provider as LightProvider,
  MINT,
  Account,
  TokenUtxoBalance,
  Balance,
  TOKEN_REGISTRY,
  BN_0,
  Utxo,
  createTestInUtxo,
} from "../src";
import { WasmFactory, LightWasm } from "@lightprotocol/account.rs";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { compareUtxos } from "./test-utils/compareUtxos";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

describe("Balance Functional", () => {
  const seed32 = bs58.encode(new Uint8Array(32).fill(1));
  const shieldAmount = 20_000;
  const shieldFeeAmount = 10_000;

  let lightWasm: LightWasm,
    lightProvider: LightProvider,
    shieldUtxo1: Utxo,
    account: Account;
  before(async () => {
    lightWasm = await WasmFactory.getInstance();
    account = Account.createFromSeed(lightWasm, seed32);
    lightProvider = await LightProvider.loadMock();
    shieldUtxo1 = createTestInUtxo({
      lightWasm,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN(shieldFeeAmount), new BN(shieldAmount)],
      account,
      merkleTreeLeafIndex: 1,
    });
  });

  it("Test Balance moveToSpentUtxos", async () => {
    const balance: Balance = {
      tokenBalances: new Map([
        [SystemProgram.programId.toBase58(), TokenUtxoBalance.initSol()],
      ]),
      totalSolBalance: BN_0,
      programBalances: new Map(),
      nftBalances: new Map(),
    };
    const tokenBalanceUsdc = new TokenUtxoBalance(TOKEN_REGISTRY.get("USDC")!);
    balance.tokenBalances.set(
      tokenBalanceUsdc.tokenData.mint.toBase58(),
      tokenBalanceUsdc,
    );

    balance.tokenBalances
      .get(MINT.toBase58())
      ?.addUtxo(shieldUtxo1.utxoHash, shieldUtxo1, "utxos");

    const utxo = balance.tokenBalances
      .get(MINT.toBase58())
      ?.utxos.get(shieldUtxo1.utxoHash);
    compareUtxos(utxo!, shieldUtxo1);
    assert.equal(
      balance.tokenBalances.get(MINT.toBase58())?.totalBalanceSol.toString(),
      shieldUtxo1.amounts[0].toString(),
    );
    assert.equal(
      balance.tokenBalances.get(MINT.toBase58())?.totalBalanceSpl.toString(),
      shieldUtxo1.amounts[1].toString(),
    );
    assert.equal(
      balance.tokenBalances.get(SystemProgram.programId.toBase58())?.spentUtxos
        .size,
      0,
    );

    balance.tokenBalances
      .get(MINT.toBase58())
      ?.moveToSpentUtxos(shieldUtxo1.utxoHash);
    assert.equal(
      balance.tokenBalances.get(MINT.toBase58())?.totalBalanceSol.toString(),
      "0",
    );
    assert.equal(
      balance.tokenBalances.get(MINT.toBase58())?.totalBalanceSpl.toString(),
      "0",
    );
    assert.equal(
      balance.tokenBalances.get(MINT.toBase58())?.spentUtxos.size,
      1,
    );

    assert.equal(balance.tokenBalances.get(MINT.toBase58())?.utxos.size, 0);

    const _shieldUtxo1 = balance.tokenBalances
      .get(MINT.toBase58())
      ?.spentUtxos.get(shieldUtxo1.utxoHash);
    compareUtxos(_shieldUtxo1!, shieldUtxo1);
  });
});
