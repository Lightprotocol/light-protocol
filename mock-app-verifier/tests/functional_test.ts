import * as anchor from "@coral-xyz/anchor";
const path = require("path");

import {
  Utxo,
  Transaction,
  ADMIN_AUTH_KEYPAIR,
  createTestAccounts,
  KEYPAIR_PRIVKEY,
  Account,
  TRANSACTION_MERKLE_TREE_KEY,
  TransactionParameters,
  Provider as LightProvider,
  userTokenAccount,
  ADMIN_AUTH_KEY,
  confirmConfig,
  Action,
  TestRelayer,
  createAccountObject,
  TestTransaction,
  IDL_VERIFIER_PROGRAM_TWO,
  User,
  airdropShieldedSol,
  MINT,
  airdropShieldedMINTSpl,
  IDL_VERIFIER_PROGRAM_ZERO,
  Provider,
  LOOK_UP_TABLE,
  ProgramParameters,
} from "@lightprotocol/zk.js";
import {
  Keypair as SolanaKeypair,
  SystemProgram,
  PublicKey,
} from "@solana/web3.js";

import { buildPoseidonOpt } from "circomlibjs";
import { BN } from "@coral-xyz/anchor";
import { it } from "mocha";
import { IDL } from "../target/types/mock_verifier";
import { assert, expect } from "chai";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

const verifierProgramId = new PublicKey(
  "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS",
);
var POSEIDON,
  RELAYER,
  KEYPAIR,
  relayerRecipientSol: PublicKey,
  outputUtxoSpl: Utxo,
  outputUtxoSol: Utxo;

const storeAndExecuteAppUtxo = async (
  seed: string,
  testInputs: any,
  airdrop: boolean,
) => {
  const lightProvider = await LightProvider.init({
    wallet: ADMIN_AUTH_KEYPAIR,
    relayer: RELAYER,
  });
  const user: User = await User.init({ provider: lightProvider, seed });

  if (airdrop) {
    if (testInputs.utxo.amounts[0]) {
      await airdropShieldedSol({
        seed: testInputs.seed,
        amount: testInputs.utxo.amounts[0].div(new BN(1e9)).toNumber(),
      });
    }
  }

  let res = await user.storeAppUtxo({
    appUtxo: testInputs.utxo,
    action: testInputs.action,
  });
  console.log("storeAppUtxo res", res);

  const { utxo, status } = await user.getUtxo(
    testInputs.utxo.getCommitment(testInputs.poseidon),
    true,
    IDL,
  );

  testInputs.utxo.index = utxo.index;
  assert.equal(status, "ready");
  Utxo.equal(testInputs.poseidon, utxo, testInputs.utxo);
  const circuitPath = path.join("build-circuit");

  const programParameters: ProgramParameters = {
    inputs: {
      releaseSlot: utxo.appData.releaseSlot,
      currentSlot: utxo.appData.releaseSlot, // for testing we can use the same value
    },
    verifierIdl: IDL,
    path: circuitPath,
  };

  await user.executeAppUtxo({
    appUtxo: utxo,
    programParameters,
    action: Action.TRANSFER,
  });
  const utxoSpent = await user.getUtxo(
    testInputs.utxo.getCommitment(testInputs.poseidon),
    true,
    IDL,
  );
  assert.equal(utxoSpent.status, "spent");
  Utxo.equal(testInputs.poseidon, utxoSpent.utxo, utxo);
};

describe("Mock verifier functional", () => {
  // Configure the client to use the local cluster.
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  const provider = anchor.AnchorProvider.local(
    "http://127.0.0.1:8899",
    confirmConfig,
  );
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
  const circuitPath = path.join("build-circuit");

  anchor.setProvider(provider);
  var poseidon, account: Account, outputUtxo: Utxo;
  const seed = bs58.encode(new Uint8Array(32).fill(1));
  let lightProvider: Provider;
  before(async () => {
    poseidon = await buildPoseidonOpt();
    await createTestAccounts(provider.connection, userTokenAccount);
    POSEIDON = await buildPoseidonOpt();
    KEYPAIR = new Account({
      poseidon: POSEIDON,
      seed: KEYPAIR_PRIVKEY.toString(),
    });

    relayerRecipientSol = SolanaKeypair.generate().publicKey;

    await provider.connection.requestAirdrop(
      relayerRecipientSol,
      2_000_000_000,
    );

    RELAYER = new TestRelayer(
      relayerPubkey: ADMIN_AUTH_KEYPAIR.publicKey,
      lookUpTable: LOOK_UP_TABLE,
      relayerRecipientSol,
      new BN(100000),
      new BN(10_000_000),
      ADMIN_AUTH_KEYPAIR,
    );
    lightProvider = await LightProvider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
    });
    account = new Account({
      poseidon,
      seed: bs58.encode(new Uint8Array(32).fill(1)),
    });
    outputUtxo = new Utxo({
      poseidon,
      assets: [SystemProgram.programId],
      account,
      amounts: [new BN(1_000_000)],
      appData: { releaseSlot: new BN(1) },
      appDataIdl: IDL,
      verifierAddress: verifierProgramId,
      index: 0,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    outputUtxoSol = new Utxo({
      poseidon,
      assets: [SystemProgram.programId],
      account,
      amounts: [new BN(1_12321211)],
      appData: { releaseSlot: new BN(1) },
      appDataIdl: IDL,
      verifierAddress: verifierProgramId,
      index: 0,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    outputUtxoSpl = new Utxo({
      poseidon,
      assets: [SystemProgram.programId, MINT],
      account,
      amounts: [new BN(1_000_000), new BN(1234)],
      appData: { releaseSlot: new BN(1) },
      appDataIdl: IDL,
      verifierAddress: verifierProgramId,
      index: 0,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
  });

  // TODO: throw an error if too much app data was provided
  it("To from bytes ", async () => {
    let bytes = await outputUtxo.toBytes();

    let utxo1 = Utxo.fromBytes({
      poseidon,
      bytes,
      index: 0,
      account,
      appDataIdl: IDL,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    Utxo.equal(poseidon, outputUtxo, utxo1);
  });

  it("Pick app data from utxo data", () => {
    let data = createAccountObject(
      {
        releaseSlot: 1,
        currentSlot: 2,
        rndOtherStuff: { s: 2342 },
        o: [2, 2, new BN(2)],
      },
      IDL.accounts,
      "utxoAppData",
    );
    assert.equal(data.releaseSlot, 1);
    assert.equal(data.currentSlot, undefined);
    assert.equal(data.rndOtherStuff, undefined);
    assert.equal(data.o, undefined);

    expect(() => {
      createAccountObject(
        { rndOtherStuff: { s: 2342 }, o: [2, 2, new BN(2)] },
        IDL.accounts,
        "utxoAppData",
      );
    }).to.throw(Error);
  });

  it("create app utxo with shield and sol ", async () => {
    const testInputsSol1 = {
      utxo: outputUtxoSol,
      action: Action.SHIELD,
      poseidon,
    };

    await storeAndExecuteAppUtxo(seed, testInputsSol1, false);
  });

  it("create app utxo with transfer and spl", async () => {
    await airdropShieldedSol({
      amount: 10,
      seed,
    });
    await airdropShieldedMINTSpl({
      amount: outputUtxoSpl.amounts[1].toNumber(),
      seed,
    });

    const testInputsSpl = {
      utxo: outputUtxoSpl,
      action: Action.TRANSFER,
      poseidon,
    };

    await storeAndExecuteAppUtxo(seed, testInputsSpl, false);
  });

  it("Test Deposit MockVerifier cpi VerifierTwo", async () => {
    let lightProvider = await LightProvider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
    });

    const txParams = new TransactionParameters({
      outputUtxos: [outputUtxo],
      transactionMerkleTreePubkey: TRANSACTION_MERKLE_TREE_KEY,
      senderSpl: userTokenAccount, // just any token account
      senderSol: ADMIN_AUTH_KEY, //
      lookUpTable: LOOK_UP_TABLE,
      poseidon,
      action: Action.SHIELD,
      encryptedUtxos: Uint8Array.from([
        ...new Uint8Array(240).fill(1),
        ...new Uint8Array(16).fill(0),
      ]), // manual padding required
      verifierIdl: IDL_VERIFIER_PROGRAM_ZERO,
    });

    let transactionTester = new TestTransaction({
      txParams,
      provider: lightProvider,
    });

    await transactionTester.getTestValues();

    let tx = new Transaction({
      provider: lightProvider,
      params: txParams,
    });

    await tx.compile();
    await tx.provider.provider.connection.confirmTransaction(
      await tx.provider.provider.connection.requestAirdrop(
        tx.params.accounts.authority,
        1_000_000_000,
      ),
    );
    await tx.getProof();
    await tx.getRootIndex();
    tx.getPdaAddresses();
    await tx.sendAndConfirmTransaction();
    await transactionTester.checkBalances(
      tx.transactionInputs,
      tx.remainingAccounts,
      tx.proofInput,
      KEYPAIR,
    );
    await lightProvider.relayer.updateMerkleTree(lightProvider);
  });

  it("Test Withdrawal MockVerifier cpi VerifierTwo", async () => {
    const poseidon = await buildPoseidonOpt();

    let lightProvider = await LightProvider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
    });

    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(relayerRecipientSol, 10000000),
    );

    // TODO: add check that recipients are defined if withdrawal
    const txParams = new TransactionParameters({
      inputUtxos: [outputUtxo],
      transactionMerkleTreePubkey: TRANSACTION_MERKLE_TREE_KEY,
      recipientSpl: userTokenAccount, // just any token account
      recipientSol: SolanaKeypair.generate().publicKey,
      action: Action.UNSHIELD,
      poseidon,
      relayer: RELAYER,
      verifierIdl: IDL_VERIFIER_PROGRAM_TWO,
    });

    const appParams = {
      inputs: { releaseSlot: new BN(1), currentSlot: new BN(1) },
      path: circuitPath,
      verifierIdl: IDL,
    };

    let transactionTester = new TestTransaction({
      txParams,
      provider: lightProvider,
      appParams,
    });
    await transactionTester.getTestValues();

    let tx = new Transaction({
      provider: lightProvider,
      params: txParams,
      appParams,
    });

    await tx.compile();
    await tx.getProof();
    await tx.getAppProof();
    await tx.getRootIndex();
    tx.getPdaAddresses();
    await tx.sendAndConfirmTransaction();
    await transactionTester.checkBalances(
      tx.transactionInputs,
      tx.remainingAccounts,
      tx.proofInput,
      KEYPAIR,
    );
  });
});
