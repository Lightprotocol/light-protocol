import * as anchor from "@coral-xyz/anchor";
import { Keypair as SolanaKeypair, Keypair } from "@solana/web3.js";
const solana = require("@solana/web3.js");
import { assert } from "chai";

import {
  createMintWrapper,
  LightMerkleTreeProgram,
  merkleTreeProgramId,
  IDL_LIGHT_MERKLE_TREE_PROGRAM,
  ADMIN_AUTH_KEYPAIR,
  MINT,
  createTestAccounts,
  userTokenAccount,
  confirmConfig,
  lightPsp2in2outId,
  MerkleTreeConfig,
  POOL_TYPE,
  IDL_LIGHT_PSP2IN2OUT,
  Provider,
  TestRelayer,
  RELAYER_FEE,
  User,
  airdropSol,
  createSolanaInstructions,
  UnshieldTransactionInput,
  getVerifierProgramId,
  getSystemProof,
  createSystemProofInputs,
  prepareAccounts,
  createUnshieldTransaction,
  getSolanaRemainingAccounts,
  Utxo,
  syncInputUtxosMerkleProofs,
} from "../../src";
import { Hasher, WasmHasher } from "@lightprotocol/account.rs";
import { getOrCreateAssociatedTokenAccount } from "@solana/spl-token";
import { Address } from "@coral-xyz/anchor";

let HASHER: Hasher, RELAYER;

console.log = () => {};
describe("Merkle Tree Tests", () => {
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.local(
    "http://127.0.0.1:8899",
    confirmConfig,
  );
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";

  anchor.setProvider(provider);
  const merkleTreeProgram: anchor.Program<LightMerkleTreeProgram> =
    new anchor.Program(IDL_LIGHT_MERKLE_TREE_PROGRAM, merkleTreeProgramId);

  const shieldAmount = 10;
  let INVALID_MERKLE_TREE_AUTHORITY_PDA,
    INVALID_SIGNER,
    lightProvider: Provider,
    merkleTreeConfig: MerkleTreeConfig;
  before(async () => {
    HASHER = await WasmHasher.getInstance();
    await createTestAccounts(provider.connection, userTokenAccount);

    const merkleTreeAccountInfoInit = await provider.connection.getAccountInfo(
      MerkleTreeConfig.getTransactionMerkleTreePda(),
    );
    console.log("merkleTreeAccountInfoInit ", merkleTreeAccountInfoInit);
    INVALID_SIGNER = Keypair.generate();
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        INVALID_SIGNER.publicKey,
        1_000_000_000_000,
      ),
      "confirmed",
    );
    INVALID_MERKLE_TREE_AUTHORITY_PDA = solana.PublicKey.findProgramAddressSync(
      [anchor.utils.bytes.utf8.encode("MERKLE_TREE_AUTHORITY_INV")],
      merkleTreeProgram.programId,
    )[0];

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
      connection: provider.connection,
      hasher: HASHER,
    });
    await airdropSol({
      connection: provider.connection,
      lamports: 1e10,
      recipientPublicKey: ADMIN_AUTH_KEYPAIR.publicKey,
    });
    lightProvider = await Provider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
      confirmConfig,
    });
    merkleTreeConfig = new MerkleTreeConfig({
      payer: ADMIN_AUTH_KEYPAIR,
      anchorProvider: provider,
    });
  });

  it("Initialize Merkle Tree Test", async () => {
    const lightPsp2in2out = new anchor.Program(
      IDL_LIGHT_PSP2IN2OUT,
      lightPsp2in2outId,
    );

    // Security Claims
    // Init authority pda
    // - can only be inited by a hardcoded pubkey
    // Update authority pda
    // - can only be invoked by current authority

    const merkleTreeAccountInfoInit = await provider.connection.getAccountInfo(
      MerkleTreeConfig.getTransactionMerkleTreePda(),
    );
    console.log("merkleTreeAccountInfoInit ", merkleTreeAccountInfoInit);
    INVALID_SIGNER = new anchor.web3.Account();
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        INVALID_SIGNER.publicKey,
        1_000_000_000_000,
      ),
      "confirmed",
    );

    INVALID_MERKLE_TREE_AUTHORITY_PDA = solana.PublicKey.findProgramAddressSync(
      [anchor.utils.bytes.utf8.encode("MERKLE_TREE_AUTHORITY_INV")],
      merkleTreeProgram.programId,
    )[0];

    merkleTreeConfig.getMerkleTreeAuthorityPda();

    let error;

    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
    try {
      await merkleTreeConfig.initMerkleTreeAuthority();
    } catch (e) {
      error = e;
    }
    merkleTreeConfig.getMerkleTreeAuthorityPda();
    console.log(error);

    assert.isTrue(
      error.logs.includes(
        "Program log: Instruction: InitializeMerkleTreeAuthority",
      ),
    );
    error = undefined;

    // init merkle tree with invalid signer
    try {
      await merkleTreeConfig.initMerkleTreeAuthority(INVALID_SIGNER);
      console.log("Registering AUTHORITY success");
    } catch (e) {
      error = e;
    }
    console.log("error ", error);
    assert.isTrue(
      error.logs.includes(
        "Program log: Instruction: InitializeMerkleTreeAuthority",
      ),
    );
    error = undefined;

    // initing real mt authority
    await merkleTreeConfig.initMerkleTreeAuthority();

    const newAuthority = Keypair.generate();
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        newAuthority.publicKey,
        1_000_000_000_000,
      ),
      "confirmed",
    );

    // update merkle tree with invalid signer
    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.updateMerkleTreeAuthority(
        newAuthority.publicKey,
        true,
      );
      console.log("Registering AUTHORITY success");
    } catch (e) {
      error = e;
    }
    console.log("InvalidAuthority ", error);

    // assert.equal(error.error.errorMessage, "InvalidAuthority");
    assert.isTrue(
      error.logs.includes(
        "Program log: AnchorError caused by account: authority. Error Code: InvalidAuthority. Error Number: 6016. Error Message: InvalidAuthority.",
      ),
    );
    error = undefined;
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;

    // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
    try {
      await merkleTreeConfig.updateMerkleTreeAuthority(
        newAuthority.publicKey,
        true,
      );
      console.log("Registering AUTHORITY success");
    } catch (e) {
      error = e;
    }
    await merkleTreeConfig.getMerkleTreeAuthorityPda();
    console.log("updateMerkleTreeAuthority ", error);

    assert.isTrue(
      error.logs.includes(
        "Program log: AnchorError caused by account: merkle_tree_authority_pda. Error Code: AccountNotInitialized. Error Number: 3012. Error Message: The program expected this account to be already initialized.",
      ),
    );
    error = undefined;

    await merkleTreeConfig.updateMerkleTreeAuthority(newAuthority.publicKey);
    merkleTreeConfig.payer = newAuthority;
    await merkleTreeConfig.updateMerkleTreeAuthority(
      ADMIN_AUTH_KEYPAIR.publicKey,
    );
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;

    // invalid signer
    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.registerVerifier(lightPsp2in2out.programId);
    } catch (e) {
      error = e;
    }
    console.log(error);

    assert.isTrue(
      error.logs.includes(
        "Program log: AnchorError caused by account: authority. Error Code: InvalidAuthority. Error Number: 6016. Error Message: InvalidAuthority.",
      ),
    );
    error = undefined;
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;

    // invalid pda
    const tmp =
      merkleTreeConfig.registeredVerifierPdas[0].registeredVerifierPda;
    merkleTreeConfig.registeredVerifierPdas[0].registeredVerifierPda =
      INVALID_SIGNER.publicKey;
    try {
      await merkleTreeConfig.registerVerifier(lightPsp2in2out.programId);
    } catch (e) {
      error = e;
    }
    console.log(error);

    // assert.equal(error.error.origin, "registered_verifier_pda");
    assert.isTrue(
      error.logs.includes("Program log: Instruction: RegisterVerifier"),
    );
    merkleTreeConfig.registeredVerifierPdas[0].registeredVerifierPda = tmp;
    error = undefined;

    const pda = merkleTreeConfig.merkleTreeAuthorityPda;
    assert.isDefined(pda);
    let merkleTreeAuthority =
      await merkleTreeProgram.account.merkleTreeAuthority.fetch(pda as Address);

    // update merkle tree with invalid signer
    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.enablePermissionlessSplTokens(true);
    } catch (e) {
      error = e;
    }

    assert.isTrue(
      error.logs.includes(
        "Program log: AnchorError caused by account: authority. Error Code: InvalidAuthority. Error Number: 6016. Error Message: InvalidAuthority.",
      ),
    );
    error = undefined;
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;

    // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
    try {
      await merkleTreeConfig.enablePermissionlessSplTokens(true);
    } catch (e) {
      error = e;
    }
    await merkleTreeConfig.getMerkleTreeAuthorityPda();

    assert.isTrue(
      error.logs.includes(
        "Program log: AnchorError caused by account: merkle_tree_authority_pda. Error Code: AccountNotInitialized. Error Number: 3012. Error Message: The program expected this account to be already initialized.",
      ),
    );
    error = undefined;

    await merkleTreeConfig.enablePermissionlessSplTokens(true);

    merkleTreeAuthority =
      await merkleTreeProgram.account.merkleTreeAuthority.fetch(
        merkleTreeConfig.merkleTreeAuthorityPda as Address,
      );
    assert.equal(merkleTreeAuthority.enablePermissionlessSplTokens, true);
    await merkleTreeConfig.enablePermissionlessSplTokens(false);
    merkleTreeAuthority =
      await merkleTreeProgram.account.merkleTreeAuthority.fetch(
        merkleTreeConfig.getMerkleTreeAuthorityPda(),
      );
    assert.equal(merkleTreeAuthority.enablePermissionlessSplTokens, false);

    // update merkle tree with invalid signer
    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.registerPoolType(new Array(32).fill(0));
    } catch (e) {
      error = e;
    }
    console.log("register pool type ", error);

    assert.isTrue(
      error.logs.some((log) =>
        log.includes(
          "Error Code: InvalidAuthority. Error Number: 6016. Error Message: InvalidAuthority.",
        ),
      ),
    );
    error = undefined;
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;

    // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
    try {
      await merkleTreeConfig.registerPoolType(new Array(32).fill(0));
    } catch (e) {
      error = e;
    }
    await merkleTreeConfig.getMerkleTreeAuthorityPda();

    assert.isTrue(
      error.logs.includes(
        "Program log: AnchorError caused by account: merkle_tree_authority_pda. Error Code: AccountNotInitialized. Error Number: 3012. Error Message: The program expected this account to be already initialized.",
      ),
    );
    error = undefined;

    await merkleTreeConfig.registerPoolType(new Array(32).fill(0));

    const registeredPoolTypePdaAccount =
      await merkleTreeProgram.account.registeredPoolType.fetch(
        merkleTreeConfig.poolTypes[0].poolPda,
      );

    assert.equal(
      registeredPoolTypePdaAccount.poolType.toString(),
      new Uint8Array(32).fill(0).toString(),
    );

    // update merkle tree with invalid signer
    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.registerSolPool(new Array(32).fill(0));
    } catch (e) {
      error = e;
    }
    console.log(error);

    assert.isTrue(
      error.logs.some((log) =>
        log.includes(
          "Error Code: InvalidAuthority. Error Number: 6016. Error Message: InvalidAuthority.",
        ),
      ),
    );
    error = undefined;
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;

    // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
    try {
      await merkleTreeConfig.registerSolPool(new Array(32).fill(0));
    } catch (e) {
      error = e;
    }
    await merkleTreeConfig.getMerkleTreeAuthorityPda();
    console.log("error ", error);

    assert.isTrue(
      error.logs.includes(
        "Program log: AnchorError caused by account: merkle_tree_authority_pda. Error Code: AccountNotInitialized. Error Number: 3012. Error Message: The program expected this account to be already initialized.",
      ),
    );
    error = undefined;

    // valid
    await merkleTreeConfig.registerSolPool(new Array(32).fill(0));
    console.log("merkleTreeConfig ", merkleTreeConfig);

    const registeredSolPdaAccount =
      await merkleTreeProgram.account.registeredAssetPool.fetch(
        MerkleTreeConfig.getSolPoolPda(merkleTreeProgramId).pda,
      );
    assert.equal(
      registeredSolPdaAccount.poolType.toString(),
      new Uint8Array(32).fill(0).toString(),
    );
    assert.equal(registeredSolPdaAccount.index.toString(), "0");
    assert.equal(
      registeredSolPdaAccount.assetPoolPubkey.toBase58(),
      MerkleTreeConfig.getSolPoolPda(merkleTreeProgramId).pda.toBase58(),
    );

    const mint = await createMintWrapper({
      authorityKeypair: ADMIN_AUTH_KEYPAIR,
      connection: provider.connection,
    });

    // update merkle tree with invalid signer
    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.registerSplPool(new Array(32).fill(0), mint);
    } catch (e) {
      error = e;
    }
    console.log(" registerSplPool ", error);

    assert.isTrue(
      error.logs.some((log) =>
        log.includes(
          "Error Code: InvalidAuthority. Error Number: 6016. Error Message: InvalidAuthority.",
        ),
      ),
    );
    error = undefined;
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;

    // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
    try {
      await merkleTreeConfig.registerSplPool(new Array(32).fill(0), mint);
    } catch (e) {
      error = e;
    }
    await merkleTreeConfig.getMerkleTreeAuthorityPda();

    assert.isTrue(
      error.logs.includes(
        "Program log: AnchorError caused by account: merkle_tree_authority_pda. Error Code: AccountNotInitialized. Error Number: 3012. Error Message: The program expected this account to be already initialized.",
      ),
    );

    // valid
    await merkleTreeConfig.registerSplPool(new Array(32).fill(0), mint);
    console.log(merkleTreeConfig.poolPdas);

    let registeredSplPdaAccount =
      await merkleTreeProgram.account.registeredAssetPool.fetch(
        merkleTreeConfig.poolPdas[0].pda,
      );
    registeredSplPdaAccount =
      await merkleTreeProgram.account.registeredAssetPool.fetch(
        merkleTreeConfig.poolPdas[merkleTreeConfig.poolPdas.length - 1].pda,
      );

    console.log(registeredSplPdaAccount);

    assert.equal(
      registeredSplPdaAccount.poolType.toString(),
      new Uint8Array(32).fill(0).toString(),
    );
    assert.equal(registeredSplPdaAccount.index.toString(), "1");
    assert.equal(
      registeredSplPdaAccount.assetPoolPubkey.toBase58(),
      merkleTreeConfig.poolPdas[
        merkleTreeConfig.poolPdas.length - 1
      ].token.toBase58(),
    );

    const merkleTreeAuthority1 =
      await merkleTreeProgram.account.merkleTreeAuthority.fetch(
        merkleTreeConfig.merkleTreeAuthorityPda as Address,
      );
    console.log(merkleTreeAuthority1);
    assert.equal(merkleTreeAuthority1.registeredAssetIndex.toString(), "2");
    await merkleTreeConfig.registerVerifier(lightPsp2in2out.programId);
    await merkleTreeConfig.registerSplPool(POOL_TYPE, MINT);
  });

  it("Switch to a new Merkle tree", async () => {
    const user = await User.init({ provider: lightProvider });
    await user.shield({
      publicAmountSpl: shieldAmount,
      publicAmountSol: shieldAmount,
      token: "USDC",
    });

    await user.getBalance();
    const unshieldUtxo: Utxo = user.getAllUtxos()[0];
    const {
      root,
      index: rootIndex,
      syncedUtxos: inputUtxos,
    } = await syncInputUtxosMerkleProofs({
      inputUtxos: [unshieldUtxo],
      merkleTreePublicKey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      relayer: lightProvider.relayer,
    });

    const tokenAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      ADMIN_AUTH_KEYPAIR,
      MINT,
      ADMIN_AUTH_KEYPAIR.publicKey,
    );
    const recipientSpl = tokenAccount.address;

    const { transactionMerkleTree, eventMerkleTree, index } =
      await merkleTreeConfig.initializeNewMerkleTrees();

    const verifierIdl = IDL_LIGHT_PSP2IN2OUT;
    const unshieldTransactionInput: UnshieldTransactionInput = {
      hasher: HASHER,
      mint: MINT,
      transactionMerkleTreePubkey:
        MerkleTreeConfig.getTransactionMerkleTreePda(),
      recipientSpl,
      recipientSol: ADMIN_AUTH_KEYPAIR.publicKey,
      relayerPublicKey: lightProvider.relayer.accounts.relayerPubkey,
      systemPspId: getVerifierProgramId(verifierIdl),
      account: user.account,
      inputUtxos,
      relayerFee: lightProvider.relayer.getRelayerFee(false),
      ataCreationFee: false,
    };

    const unshieldTransaction = await createUnshieldTransaction(
      unshieldTransactionInput,
    );

    const systemProofInputs = createSystemProofInputs({
      transaction: unshieldTransaction,
      hasher: HASHER,
      account: user.account,
      root,
    });
    const systemProof = await getSystemProof({
      account: user.account,
      inputUtxos: unshieldTransaction.private.inputUtxos,
      verifierIdl,
      systemProofInputs,
    });

    const remainingMerkleTreeAccounts = {
      nextTransactionMerkleTree: {
        isSigner: false,
        isWritable: true,
        pubkey: transactionMerkleTree,
      },
      nextEventMerkleTree: {
        isSigner: false,
        isWritable: true,
        pubkey: eventMerkleTree,
      },
    };

    const remainingSolanaAccounts = getSolanaRemainingAccounts(
      systemProof.parsedPublicInputsObject as any,
      remainingMerkleTreeAccounts,
    );

    const accounts = prepareAccounts({
      transactionAccounts: unshieldTransaction.public.accounts,
      eventMerkleTreePubkey: MerkleTreeConfig.getEventMerkleTreePda(),
      relayerRecipientSol: lightProvider.relayer.accounts.relayerRecipientSol,
      signer: lightProvider.relayer.accounts.relayerPubkey,
    });
    // createSolanaInstructionsWithAccounts
    const instructions = await createSolanaInstructions({
      action: unshieldTransaction.action,
      rootIndex,
      systemProof,
      remainingSolanaAccounts,
      accounts,
      publicTransactionVariables: unshieldTransaction.public,
      systemPspIdl: verifierIdl,
    });

    await lightProvider.sendAndConfirmTransaction(instructions);
  });
});
