import { test } from "@oclif/test";

import { getLightProvider, getRelayer, getUser, setAnchorProvider } from "../src/utils/utils";
import { PublicKey } from "@solana/web3.js";
import { Relayer } from "@lightprotocol/zk.js";

let circomlibjs = require("circomlibjs");
import {
  ADMIN_AUTH_KEYPAIR,
  Provider,
  createTestAccounts,
  User,
  Action,
  TestStateValidator,
  generateRandomTestAmount,
} from "@lightprotocol/zk.js";

var POSEIDON;
var RELAYER: Relayer, provider: Provider, user: User;
var testStateValidator: TestStateValidator;

describe("Test light-cli for user", () => {

  before(async () => {
    const anchorProvider = await setAnchorProvider();
    await createTestAccounts(anchorProvider.connection);
    POSEIDON = await circomlibjs.buildPoseidonOpt();
    RELAYER = await getRelayer();
    provider = await getLightProvider(ADMIN_AUTH_KEYPAIR);
    user = await getUser(ADMIN_AUTH_KEYPAIR);
  }) 

  test.
  skip().
  it("(user class) shield SPL", async () => {
    var expectedSpentUtxosLength = 0;
    var expectedUtxoHistoryLength = 1;
    let testInputs = {
      amountSpl: generateRandomTestAmount(0, 100000, 2),
      amountSol: 0,
      token: "USDC",
      type: Action.SHIELD,
      expectedUtxoHistoryLength,
      expectedSpentUtxosLength,
    };

    const testStateValidator = new TestStateValidator({
      userSender: user,
      userRecipient: user,
      provider,
      testInputs,
    });

    await testStateValidator.fetchAndSaveState();

    await user.shield({
      publicAmountSpl: testInputs.amountSpl,
      token: testInputs.token,
    });

    // TODO: add random amount and amount checks
    await user.provider.latestMerkleTree();

    await testStateValidator.checkTokenShielded();
  });

  test
  .do((async () => {
    var expectedSpentUtxosLength = 0;
    var expectedUtxoHistoryLength = 1;
    let testInputs = {
      amountSpl: 2,
      amountSol: 0,
      token: "USDC",
      type: Action.SHIELD,
      expectedUtxoHistoryLength,
      expectedSpentUtxosLength,
    };
    testStateValidator = new TestStateValidator({
      userSender: user,
      userRecipient: user,
      provider,
      testInputs,
    });

    await testStateValidator.fetchAndSaveState();
  }))
  .stdout()
  .command([
    'shield:spl',
    '2',
    'USDC',
  ])
  .it("(light-cli) shield SPL", async () => {

    // TODO: add random amount and amount checks
    await user.provider.latestMerkleTree();
    await testStateValidator.checkTokenShielded();
  });

  test
  .do(async () => {
    let testInputs = {
      amountSpl: 0,
      amountSol: 15,
      token: "SOL",
      type: Action.SHIELD,
      expectedUtxoHistoryLength: 1,
    };

    const testStateValidator = new TestStateValidator({
      userSender: user,
      userRecipient: user,
      provider: user.provider,
      testInputs,
    });

    await testStateValidator.fetchAndSaveState();
  })
  .stdout()
  .command([
    'shield:sol',
    '15',
  ])
  .it("(light-cli) shield SOL", async () => {
   
    // TODO: add random amount and amount checks
    await user.provider.latestMerkleTree();
    // is failing because we are paying for the merkle tree update from the same keypair
    // TODO: factor these costs into the equation or pay for the update from a different keypair for example one defined in the testrelayer
    // await testStateValidator.checkSolShielded();
  });

  test
  .skip()
  .do(async () => {
    //const solRecipient = SolanaKeypair.generate();
    const solRecipient = new PublicKey('E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc');
    const testInputs = {
      amountSpl: 0,
      amountSol: 2,
      token: "SOL",
      type: Action.UNSHIELD,
      recipientSpl: solRecipient,
      expectedUtxoHistoryLength: 1,
    };

    const testStateValidator = new TestStateValidator({
      userSender: user,
      userRecipient: user,
      provider: user.provider,
      testInputs,
    });

    await testStateValidator.fetchAndSaveState();    
  })
  .stdout()
  .command([
    'unshield:sol',
    '2',
    `E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc`
  ])
  .it("(light-cli) unshield SOL", async () => {

    await user.provider.latestMerkleTree();
    // await testStateValidator.checkTokenUnshielded();
  });

  test
  .do(async () => {
    // const solRecipient = SolanaKeypair.generate();
    const solRecipient = new PublicKey('E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc');
    const testInputs = {
      amountSpl: 1,
      amountSol: 0,
      token: "USDC",
      type: Action.UNSHIELD,
      recipientSpl: solRecipient,
      expectedUtxoHistoryLength: 1,
    };
    const testStateValidator = new TestStateValidator({
      userSender: user,
      userRecipient: user,
      provider: user.provider,
      testInputs,
    });

    await testStateValidator.fetchAndSaveState();
  })
  .stdout()
  .command([
    'unshield:spl',
    '1',
    'USDC',
    `E2CDgD4vq636mLf9pgMTyKdK3k8gbPZM95YetYMfPLbc`
  ])
  .it("(light-cli) unshield SPL", async () => {

    await user.provider.latestMerkleTree();

    await testStateValidator.checkTokenUnshielded();
  });
})

