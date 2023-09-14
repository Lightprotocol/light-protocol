import { assert } from "chai";
import {
  SystemProgram,
  Keypair as SolanaKeypair,
  PublicKey,
  Keypair,
} from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { it } from "mocha";
const circomlibjs = require("circomlibjs");
const { buildPoseidonOpt } = circomlibjs;
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
// Load chai-as-promised support
chai.use(chaiAsPromised);

import {
  FEE_ASSET,
  Provider as LightProvider,
  MINT,
  Relayer,
  Utxo,
  Account,
  TokenUtxoBalance,
  Balance,
  TOKEN_REGISTRY,
  ParsedIndexedTransaction,
  User,
  NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
  MerkleTreeConfig,
  BN_0,
} from "../src";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

describe("Utxo Functional", () => {
  let seed32 = bs58.encode(new Uint8Array(32).fill(1));
  let depositAmount = 20_000;
  let depositFeeAmount = 10_000;

  let mockPubkey = SolanaKeypair.generate().publicKey;
  let mockPubkey3 = SolanaKeypair.generate().publicKey;
  let poseidon: any, lightProvider, deposit_utxo1: Utxo, relayer, keypair;
  before(async () => {
    poseidon = await buildPoseidonOpt();
    // TODO: make fee mandatory
    relayer = new Relayer(mockPubkey3, mockPubkey, new BN(5000));
    keypair = new Account({ poseidon: poseidon, seed: seed32 });
    lightProvider = await LightProvider.loadMock();
    deposit_utxo1 = new Utxo({
      poseidon: poseidon,
      assets: [FEE_ASSET, MINT],
      amounts: [new BN(depositFeeAmount), new BN(depositAmount)],
      account: keypair,
      index: 1,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
  });

  it("Test Balance moveToSpentUtxos", async () => {
    let balance: Balance = {
      tokenBalances: new Map([
        [SystemProgram.programId.toBase58(), TokenUtxoBalance.initSol()],
      ]),
      totalSolBalance: BN_0,
      programBalances: new Map(),
      nftBalances: new Map(),
    };
    let tokenBalanceUsdc = new TokenUtxoBalance(TOKEN_REGISTRY.get("USDC")!);
    balance.tokenBalances.set(
      tokenBalanceUsdc.tokenData.mint.toBase58(),
      tokenBalanceUsdc,
    );

    balance.tokenBalances
      .get(MINT.toBase58())
      ?.addUtxo(deposit_utxo1.getCommitment(poseidon), deposit_utxo1, "utxos");

    Utxo.equal(
      poseidon,
      await balance.tokenBalances
        .get(MINT.toBase58())
        ?.utxos.get(deposit_utxo1.getCommitment(poseidon))!,
      await deposit_utxo1,
    );
    assert.equal(
      balance.tokenBalances.get(MINT.toBase58())?.totalBalanceSol.toString(),
      deposit_utxo1.amounts[0].toString(),
    );
    assert.equal(
      balance.tokenBalances.get(MINT.toBase58())?.totalBalanceSpl.toString(),
      deposit_utxo1.amounts[1].toString(),
    );
    assert.equal(
      balance.tokenBalances.get(SystemProgram.programId.toBase58())?.spentUtxos
        .size,
      0,
    );

    balance.tokenBalances
      .get(MINT.toBase58())
      ?.moveToSpentUtxos(deposit_utxo1.getCommitment(poseidon));
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

    Utxo.equal(
      poseidon,
      await balance.tokenBalances
        .get(MINT.toBase58())
        ?.spentUtxos.get(deposit_utxo1.getCommitment(poseidon))!,
      await deposit_utxo1,
    );
  });

  // this test is a mock of the syncState function
  // TODO: add a direct test
  it("Test Decrypt Balance 2 and 4 utxos", async () => {
    const provider = await LightProvider.loadMock();
    let verifierProgramLookupTable =
      provider.lookUpTables.verifierProgramLookupTable;
    let assetLookupTable = provider.lookUpTables.assetLookupTable;
    const account = new Account({ poseidon: poseidon, seed: seed32 });
    for (let j = 2; j < 4; j += 2) {
      let utxos: Utxo[] = [];
      let encryptedUtxos: any[] = [];
      for (let index = 0; index < j; index++) {
        const depositAmount = index;
        const depositFeeAmount = index;
        const utxo = new Utxo({
          poseidon: poseidon,
          assets: [FEE_ASSET, MINT],
          amounts: [new BN(depositFeeAmount), new BN(depositAmount)],
          account: account,
          index: index,
          assetLookupTable: provider.lookUpTables.assetLookupTable,
          verifierProgramLookupTable:
            provider.lookUpTables.verifierProgramLookupTable,
          blinding: new BN(1),
        });
        utxos.push(utxo);
        encryptedUtxos = [
          ...encryptedUtxos,
          ...(await utxo.encrypt(
            poseidon,
            MerkleTreeConfig.getTransactionMerkleTreePda(),
            true,
          )),
        ];
      }
      let indexedTransactions = [
        {
          leaves: utxos.map((utxo) =>
            new BN(utxo.getCommitment(poseidon)).toBuffer("le", 32),
          ),
          firstLeafIndex: "0",
          encryptedUtxos,
        },
      ];
      let decryptedUtxos: Array<Utxo | null> = new Array<Utxo | null>();
      for (const trx of indexedTransactions) {
        let leftLeafIndex = new BN(trx.firstLeafIndex).toNumber();

        for (let index = 0; index < trx.leaves.length; index += 2) {
          const leafLeft = trx.leaves[index];
          const leafRight = trx.leaves[index + 1];
          let decryptedUtxo = await Utxo.decrypt({
            poseidon,
            encBytes: Buffer.from(
              trx.encryptedUtxos.slice(
                (index / 2) * 240,
                (index / 2) * 240 + NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH,
              ),
            ),
            account,
            index: leftLeafIndex + index,
            commitment: leafLeft,
            aes: true,
            merkleTreePdaPublicKey:
              MerkleTreeConfig.getTransactionMerkleTreePda(),
            verifierProgramLookupTable,
            assetLookupTable,
          });
          decryptedUtxos.push(decryptedUtxo);
          decryptedUtxo = await Utxo.decrypt({
            poseidon,
            encBytes: Buffer.from(
              trx.encryptedUtxos.slice(
                (index / 2) * 240 + 120,
                (index / 2) * 240 +
                  NACL_ENCRYPTED_COMPRESSED_UTXO_BYTES_LENGTH +
                  120,
              ),
            ),
            account,
            index: leftLeafIndex + index + 1,
            commitment: leafRight,
            aes: true,
            merkleTreePdaPublicKey:
              MerkleTreeConfig.getTransactionMerkleTreePda(),
            verifierProgramLookupTable,
            assetLookupTable,
          });
          decryptedUtxos.push(decryptedUtxo);
        }
      }
      utxos.map((utxo, index) => {
        Utxo.equal(poseidon, utxo, decryptedUtxos[index]!);
      });
    }
  });
});
