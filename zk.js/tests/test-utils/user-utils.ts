import { Keypair, PublicKey } from "@solana/web3.js";
import {
  Action,
  Provider,
  TestRelayer,
  UserTestAssertHelper,
  User,
  TestInputs,
  confirmConfig,
} from "../../src";

export type EnvironmentConfig = {
  relayer?: TestRelayer;
  providerSolanaKeypair?: Keypair;
  poseidon?: any;
  lookUpTable?: PublicKey;
};

export async function performShielding({
  numberOfShields = 1,
  testInputs,
  environmentConfig,
}: {
  numberOfShields: number;
  testInputs: TestInputs;
  environmentConfig: EnvironmentConfig;
}) {
  if (!testInputs.recipientSeed && testInputs.shieldToRecipient)
    throw new Error("testinputs recipientSeed is undefined");
  for (let i = 0; i < numberOfShields; i++) {
    const provider = await Provider.init({
      wallet: environmentConfig.providerSolanaKeypair!,
      relayer: environmentConfig.relayer,
      confirmConfig,
    });
    const userSender = await User.init({
      provider,
    });
    const userRecipient = testInputs.shieldToRecipient
      ? await User.init({
          provider,
          seed: testInputs.recipientSeed,
        })
      : userSender;
    const testStateValidator = new UserTestAssertHelper({
      userSender,
      userRecipient,
      provider,
      testInputs,
    });
    await testStateValidator.fetchAndSaveState();

    if (testInputs.shieldToRecipient) {
      await userSender.shield({
        publicAmountSol: testInputs.amountSol,
        publicAmountSpl: testInputs.amountSpl,
        token: testInputs.token,
        recipient: userRecipient.account.getPublicKey(),
      });
    } else {
      await userSender.shield({
        publicAmountSol: testInputs.amountSol,
        publicAmountSpl: testInputs.amountSpl,
        token: testInputs.token,
      });
    }
    await userRecipient.provider.latestMerkleTree();
    if (testInputs.token === "SOL" && testInputs.type === Action.SHIELD) {
      // await testStateValidator.checkSolShielded();
    } else if (
      testInputs.token !== "SOL" &&
      testInputs.type === Action.SHIELD
    ) {
      await testStateValidator.checkSplShielded();
    } else {
      throw new Error(`No test option found for testInputs ${testInputs}`);
    }
    testInputs.expectedUtxoHistoryLength++;
  }
}

export async function performMergeAll({
  testInputs,
  environmentConfig,
}: {
  testInputs: TestInputs;
  environmentConfig: EnvironmentConfig;
}) {
  if (!testInputs.recipientSeed)
    throw new Error("testinputs recipientSeed is undefined");
  const provider = await Provider.init({
    wallet: environmentConfig.providerSolanaKeypair!,
    relayer: environmentConfig.relayer,
    confirmConfig,
  });

  const userSender: User = await User.init({
    provider,
    seed: testInputs.recipientSeed,
  });
  await userSender.getUtxoInbox();

  const testStateValidator = new UserTestAssertHelper({
    userSender,
    userRecipient: userSender,
    provider,
    testInputs,
  });

  await testStateValidator.fetchAndSaveState();
  await userSender.mergeAllUtxos(testStateValidator.tokenCtx.mint);

  /**
   * Test:
   * - if user utxo were less than 10 before, there is only one balance utxo of asset all others have been merged
   * - min(10 - nrPreBalanceUtxos[asset], nrPreBalanceInboxUtxos[asset]) have been merged thus size of utxos is less by that number
   * -
   */
  // TODO: add random amount and amount checks
  await userSender.provider.latestMerkleTree();
  await testStateValidator.checkMergedAll();
}

export async function performMergeUtxos({
  testInputs,
  environmentConfig,
}: {
  testInputs: TestInputs;
  environmentConfig: EnvironmentConfig;
}) {
  if (!testInputs.recipientSeed)
    throw new Error("testinputs recipientSeed is undefined");

  const provider = await Provider.init({
    wallet: environmentConfig.providerSolanaKeypair!,
    relayer: environmentConfig.relayer,
    confirmConfig,
  });

  const userSender: User = await User.init({
    provider,
    seed: testInputs.recipientSeed,
  });
  await userSender.getUtxoInbox();

  const testStateValidator = new UserTestAssertHelper({
    userSender,
    userRecipient: userSender,
    provider,
    testInputs,
  });

  await testStateValidator.fetchAndSaveState();
  await userSender.mergeUtxos(
    testInputs.utxoCommitments!,
    testStateValidator.tokenCtx.mint,
  );

  /**
   * Test:
   * - if user utxo were less than 10 before, there is only one balance utxo of asset all others have been merged
   * - min(10 - nrPreBalanceUtxos[asset], nrPreBalanceInboxUtxos[asset]) have been merged thus size of utxos is less by that number
   * -
   */
  // TODO: add random amount and amount checks
  await userSender.provider.latestMerkleTree();
  await testStateValidator.checkMerged();
}
