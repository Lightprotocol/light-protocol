import { assert, expect } from "chai";
import { SystemProgram, Keypair as SolanaKeypair, PublicKey } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { it } from "mocha";
import { buildPoseidonOpt } from "circomlibjs";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");

// Load chai-as-promised support
chai.use(chaiAsPromised);
import {
  FEE_ASSET,
  hashAndTruncateToCircuit,
  Provider as LightProvider,
  MINT,
  Relayer,
  UtxoError,
  UtxoErrorCode,
  Utxo,
  Account,
  newNonce,
  MERKLE_TREE_KEY,
  verifierProgramTwoProgramId,
  TokenUtxoBalance,
  Balance,
  TOKEN_REGISTRY,
  assetLookupTable,
  merkleTreeProgramId,
  decryptAddUtxoToBalance,
} from "../src";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

describe("Utxo Functional", () => {
  let seed32 = bs58.encode(new Uint8Array(32).fill(1));
  let depositAmount = 20_000;
  let depositFeeAmount = 10_000;

  let mockPubkey = SolanaKeypair.generate().publicKey;
  let mockPubkey1 = SolanaKeypair.generate().publicKey;
  let mockPubkey2 = SolanaKeypair.generate().publicKey;
  let mockPubkey3 = SolanaKeypair.generate().publicKey;
  let poseidon, lightProvider, deposit_utxo1: Utxo , outputUtxo, relayer, keypair;
  before(async () => {
    poseidon = await buildPoseidonOpt();
    // TODO: make fee mandatory
    relayer = new Relayer(
      mockPubkey3,
      mockPubkey,
      mockPubkey,
      new anchor.BN(5000),
    );
    keypair = new Account({ poseidon: poseidon, seed: seed32 });
    lightProvider = await LightProvider.loadMock();
    deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: keypair,
      index: 1,
    });
  });

  it("Test Balance mapping", async ()=> {


    let balance: Balance = {
      tokenBalances: new Map([[SystemProgram.programId.toBase58(), TokenUtxoBalance.initSol()]]),
      transactionNonce: 0,
      committedTransactionNonce:0,
    };
    let tokenBalanceUsdc = new TokenUtxoBalance(TOKEN_REGISTRY.get("USDC"));
    balance.tokenBalances.set(tokenBalanceUsdc.tokenData.mint.toBase58(), tokenBalanceUsdc);
    
    balance.tokenBalances.get(MINT.toBase58())?.addUtxo(deposit_utxo1.getCommitment(poseidon), deposit_utxo1, 'utxos');
    
    Utxo.equal(poseidon,
        await balance.tokenBalances.get(MINT.toBase58())?.utxos.get(deposit_utxo1.getCommitment(poseidon))!,
        await deposit_utxo1
    );
    assert.equal(balance.tokenBalances.get(MINT.toBase58())?.totalBalanceSol.toString(), deposit_utxo1.amounts[0].toString());
    assert.equal(balance.tokenBalances.get(MINT.toBase58())?.totalBalanceSpl.toString(), deposit_utxo1.amounts[1].toString());
    balance.tokenBalances.get(MINT.toBase58())?.movetToCommittedUtxos(deposit_utxo1.getCommitment(poseidon));
    Utxo.equal(poseidon,
        await balance.tokenBalances.get(MINT.toBase58())?.committedUtxos.get(deposit_utxo1.getCommitment(poseidon))!,
        await deposit_utxo1
    );

    assert.equal(balance.tokenBalances.get(SystemProgram.programId.toBase58())?.utxos.size, 0);

    assert.equal(balance.tokenBalances.get(MINT.toBase58())?.utxos.size, 0);

    assert.equal(balance.tokenBalances.get(MINT.toBase58())?.committedUtxos.size, 1);

    assert.equal(balance.tokenBalances.get(MINT.toBase58())?.totalBalanceSol.toString(), "0");
    assert.equal(balance.tokenBalances.get(MINT.toBase58())?.totalBalanceSpl.toString(), "0");
    balance.tokenBalances.get(MINT.toBase58())?.movetToSpentUtxos(deposit_utxo1.getCommitment(poseidon));
    assert.equal(balance.tokenBalances.get(MINT.toBase58())?.committedUtxos.size, 0);
    Utxo.equal(poseidon,
        await balance.tokenBalances.get(MINT.toBase58())?.spentUtxos.get(deposit_utxo1.getCommitment(poseidon))!,
        await deposit_utxo1
    );
  })

  it.skip("Test decryptAddUtxoToBalance ", async ()=> {
    let balance: Balance = {
      tokenBalances: new Map([[SystemProgram.programId.toBase58(), TokenUtxoBalance.initSol()]]),
      transactionNonce: 0,
      committedTransactionNonce:0,
    };
    let encBytes = await deposit_utxo1.encrypt(poseidon, merkleTreeProgramId, balance.transactionNonce);
    let derypted = await  Utxo.decrypt({
      poseidon,
      aes: true,
      encBytes,
      commitment: (new anchor.BN(deposit_utxo1.getCommitment(poseidon))).toBuffer("le", 32),
      transactionNonce: balance.transactionNonce,
      index: 0,
      merkleTreePdaPublicKey: merkleTreeProgramId,
      account: keypair
    })
    Utxo.equal(poseidon, derypted!, deposit_utxo1, true);
    await decryptAddUtxoToBalance({
      encBytes,
      index: 0,
      commitment: (new anchor.BN(deposit_utxo1.getCommitment(poseidon))).toBuffer("le", 32),
      account: keypair,
      poseidon,
      aes: true,
      balance,
      merkleTreePdaPublicKey: merkleTreeProgramId,
      // @ts-ignore: test wont work anymore with checked nullifier
      connection: {},
    })

    assert.equal(balance.transactionNonce, 1);
    Utxo.equal(poseidon, balance.tokenBalances.get(deposit_utxo1.assets[1].toBase58())?.utxos.get(deposit_utxo1.getCommitment(poseidon))!, derypted!);
  })
})
