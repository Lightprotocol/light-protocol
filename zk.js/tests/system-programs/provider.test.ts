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
  LegacyTransaction as Transaction,
  TransactionParameters,
  userTokenAccount,
  IDL_LIGHT_PSP2IN2OUT,
  airdropSol,
  MerkleTreeConfig,
  RELAYER_FEE,
} from "../../src";

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
import { WasmHasher, Hasher } from "@lightprotocol/account.rs";
import { BN, AnchorProvider, setProvider } from "@coral-xyz/anchor";

let HASHER: Hasher, KEYPAIR: Account;
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

    HASHER = await WasmHasher.getInstance();
    KEYPAIR = new Account({
      hasher: HASHER,
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
    assert(lightProviderMock.hasher);
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
      hasher: HASHER,
      assets: [SystemProgram.programId, MINT],
      amounts: [new BN(shieldFeeAmount), new BN(shieldAmount)],
      publicKey: KEYPAIR.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });
    const shieldUtxo2 = new Utxo({
      hasher: HASHER,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      publicKey: KEYPAIR.pubkey,
    });

    const txParams = new TransactionParameters({
      outputUtxos: [shieldUtxo1, shieldUtxo2],
      eventMerkleTreePubkey: MerkleTreeConfig.getEventMerkleTreePda(),
      transactionMerkleTreePubkey:
        MerkleTreeConfig.getTransactionMerkleTreePda(),
      senderSpl: userTokenAccount,
      senderSol: ADMIN_AUTH_KEYPAIR.publicKey,
      hasher: HASHER,
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
      lightProvider.hasher,
      KEYPAIR,
    );

    try {
      const res = await lightProvider.sendAndConfirmTransaction(instructions);
      console.log(res);
    } catch (e) {
      console.log(e);
    }

    assert.equal(
      lightProvider.solMerkleTree!.merkleTree.indexOf(
        shieldUtxo1.getCommitment(HASHER),
      ),
      -1,
    );
    assert.equal(
      lightProvider.solMerkleTree!.merkleTree.indexOf(
        shieldUtxo2.getCommitment(HASHER),
      ),
      -1,
    );

    await lightProvider.latestMerkleTree();
    assert.equal(
      lightProvider.solMerkleTree!.merkleTree.indexOf(
        shieldUtxo1.getCommitment(HASHER),
      ),
      0,
    );
    assert.equal(
      lightProvider.solMerkleTree!.merkleTree.indexOf(
        shieldUtxo2.getCommitment(HASHER),
      ),
      1,
    );
  });
});
