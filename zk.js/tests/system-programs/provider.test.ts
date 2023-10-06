import {
  Connection,
  Keypair as SolanaKeypair,
  SystemProgram,
} from "@solana/web3.js";
import { assert } from "chai";
import {
  Account,
  Action,
  KEYPAIR_PRIVKEY,
  Provider as LightProvider,
  Transaction,
  TransactionParameters,
  userTokenAccount,
  IDL_LIGHT_PSP2IN2OUT,
  airdropSol,
  MerkleTreeConfig,
  RELAYER_FEE,
} from "../../src";

const circomlibjs = require("circomlibjs");

// TODO: add and use  namespaces in SDK
import {
  Utxo,
  ADMIN_AUTH_KEYPAIR,
  MINT,
  Provider,
  createTestAccounts,
  confirmConfig,
  DEFAULT_ZERO,
  TestRelayer,
} from "../../src";

import { BN, AnchorProvider, setProvider } from "@coral-xyz/anchor";

let POSEIDON: any, KEYPAIR: Account;
let RELAYER: TestRelayer;

// TODO: remove deprecated function calls
describe("verifier_program", () => {
  // Configure the client to use the local cluster.
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";

  const provider = AnchorProvider.local("http://127.0.0.1:8899", confirmConfig);
  setProvider(provider);

  const userKeypair = ADMIN_AUTH_KEYPAIR;

  before("init test setup Merkle tree lookup table etc ", async () => {
    await createTestAccounts(provider.connection);

    POSEIDON = await circomlibjs.buildPoseidonOpt();
    KEYPAIR = new Account({
      poseidon: POSEIDON,
      seed: KEYPAIR_PRIVKEY.toString(),
    });

    const relayerRecipientSol = SolanaKeypair.generate().publicKey;

    await provider.connection.requestAirdrop(
      relayerRecipientSol,
      2_000_000_000,
    );

    RELAYER = new TestRelayer({
      relayerPubkey: ADMIN_AUTH_KEYPAIR.publicKey,
      relayerRecipientSol,
      relayerFee: RELAYER_FEE,
      payer: ADMIN_AUTH_KEYPAIR,
    });
  });

  it("Provider", async () => {
    const connection = new Connection("http://127.0.0.1:8899", "confirmed");
    await connection.confirmTransaction(
      await connection.requestAirdrop(
        ADMIN_AUTH_KEYPAIR.publicKey,
        10_000_000_0000,
      ),
      "confirmed",
    );
    const mockKeypair = SolanaKeypair.generate();
    await airdropSol({
      connection: provider.connection,
      lamports: 1e9,
      recipientPublicKey: mockKeypair.publicKey,
    });
    const lightProviderMock = await LightProvider.init({
      wallet: mockKeypair,
      relayer: RELAYER,
      confirmConfig,
    });
    assert.equal(lightProviderMock.wallet.isNodeWallet, true);
    assert.equal(
      lightProviderMock.wallet?.publicKey.toBase58(),
      mockKeypair.publicKey.toBase58(),
    );
    assert.equal(lightProviderMock.url, "http://127.0.0.1:8899");
    assert(lightProviderMock.poseidon);
    assert.equal(
      lightProviderMock.solMerkleTree?.pubkey.toBase58(),
      MerkleTreeConfig.getTransactionMerkleTreePda().toBase58(),
    );
    assert.equal(lightProviderMock.solMerkleTree?.merkleTree.levels, 18);
    assert.equal(
      lightProviderMock.solMerkleTree?.merkleTree.zeroElement,
      DEFAULT_ZERO,
    );
    assert.equal(
      lightProviderMock.solMerkleTree?.merkleTree._layers[0].length,
      0,
    );
  });

  it("Fetch latestMerkleTree", async () => {
    const lightProvider = await Provider.init({
      wallet: userKeypair,
      relayer: RELAYER,
      confirmConfig,
    }); // userKeypair

    const shieldFeeAmount = 10000;
    const shieldAmount = 0;

    const shieldUtxo1 = new Utxo({
      poseidon: POSEIDON,
      assets: [SystemProgram.programId, MINT],
      amounts: [new BN(shieldFeeAmount), new BN(shieldAmount)],
      publicKey: KEYPAIR.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });
    const shieldUtxo2 = new Utxo({
      poseidon: POSEIDON,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
      publicKey: KEYPAIR.pubkey,
    });

    const txParams = new TransactionParameters({
      outputUtxos: [shieldUtxo1, shieldUtxo2],
      eventMerkleTreePubkey: MerkleTreeConfig.getEventMerkleTreePda(),
      transactionMerkleTreePubkey:
        MerkleTreeConfig.getTransactionMerkleTreePda(),
      senderSpl: userTokenAccount,
      senderSol: ADMIN_AUTH_KEYPAIR.publicKey,
      poseidon: POSEIDON,
      action: Action.SHIELD,
      verifierIdl: IDL_LIGHT_PSP2IN2OUT,
      account: KEYPAIR,
    });
    const { rootIndex: rootIndex1, remainingAccounts: remainingAccounts1 } =
      await lightProvider.getRootIndex();
    const tx = new Transaction({
      rootIndex: rootIndex1,
      nextTransactionMerkleTree: remainingAccounts1.nextTransactionMerkleTree,
      solMerkleTree: lightProvider.solMerkleTree!,
      params: txParams,
    });
    const instructions = await tx.compileAndProve(
      lightProvider.poseidon,
      KEYPAIR,
    );

    try {
      const res = await lightProvider.sendAndConfirmTransaction(instructions);
      console.log(res);
    } catch (e) {
      console.log(e);
    }
    // TODO: add random amount and amount checks
    try {
      console.log("updating merkle tree...");
      const initLog = console.log;
      console.log = () => {};
      await lightProvider.relayer.updateMerkleTree(
        lightProvider,
        // provider.provider,
      );
      console.log = initLog;
      console.log("✔️updated merkle tree!");
    } catch (e) {
      console.log(e);
      throw new Error("Failed to update merkle tree!");
    }
    assert.equal(
      lightProvider.solMerkleTree!.merkleTree.indexOf(
        shieldUtxo1.getCommitment(POSEIDON),
      ),
      -1,
    );
    assert.equal(
      lightProvider.solMerkleTree!.merkleTree.indexOf(
        shieldUtxo2.getCommitment(POSEIDON),
      ),
      -1,
    );

    await lightProvider.latestMerkleTree();
    assert.equal(
      lightProvider.solMerkleTree!.merkleTree.indexOf(
        shieldUtxo1.getCommitment(POSEIDON),
      ),
      0,
    );
    assert.equal(
      lightProvider.solMerkleTree!.merkleTree.indexOf(
        shieldUtxo2.getCommitment(POSEIDON),
      ),
      1,
    );
  });
});
