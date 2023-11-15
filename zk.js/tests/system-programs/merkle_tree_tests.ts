import * as anchor from "@coral-xyz/anchor";
import {
  SystemProgram,
  Keypair as SolanaKeypair,
  Keypair,
  PublicKey,
} from "@solana/web3.js";
const solana = require("@solana/web3.js");
import { assert } from "chai";
const token = require("@solana/spl-token");

import {
  Poseidon,
  Transaction,
  Utxo,
  createMintWrapper,
  LightMerkleTreeProgram,
  merkleTreeProgramId,
  IDL_LIGHT_MERKLE_TREE_PROGRAM,
  ADMIN_AUTH_KEYPAIR,
  MINT,
  KEYPAIR_PRIVKEY,
  createTestAccounts,
  userTokenAccount,
  FEE_ASSET,
  confirmConfig,
  TransactionParameters,
  SolMerkleTree,
  lightPsp2in2outId,
  MerkleTreeConfig,
  DEFAULT_PROGRAMS,
  checkMerkleTreeUpdateStateCreated,
  executeMerkleTreeUpdateTransactions,
  newAccountWithLamports,
  checkMerkleTreeBatchUpdateSuccess,
  POOL_TYPE,
  IDL_LIGHT_PSP2IN2OUT,
  Account,
  Provider,
  Action,
  TestRelayer,
  executeUpdateMerkleTreeTransactions,
  RELAYER_FEE,
  BN_1,
  BN_0,
  BN_2,
  closeMerkleTreeUpdateState,
} from "../../src";
import { SPL_NOOP_ADDRESS } from "@solana/spl-account-compression";
import {
  getOrCreateAssociatedTokenAccount,
} from "@solana/spl-token";
import {Address} from "@coral-xyz/anchor";

let POSEIDON: Poseidon, RELAYER, KEYPAIR, deposit_utxo1;

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

  let INVALID_MERKLE_TREE_AUTHORITY_PDA, INVALID_SIGNER;
  before(async () => {
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
    const merkleTreeConfig = new MerkleTreeConfig({
      payer: ADMIN_AUTH_KEYPAIR,
      anchorProvider: provider,
    });
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
    let merkleTreeAuthority = await merkleTreeProgram.account.merkleTreeAuthority.fetch(pda as Address);

    // update lock duration with invalid signer

    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.updateLockDuration(123);
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
      await merkleTreeConfig.updateLockDuration(123);
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

    await merkleTreeConfig.updateLockDuration(123);

    await merkleTreeConfig.updateLockDuration(10);

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
        merkleTreeConfig.merkleTreeAuthorityPda as Address,
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

    // let nftMint = await createMintWrapper({authorityKeypair: ADMIN_AUTH_KEYPAIR, nft: true, connection: provider.connection})

    // var userTokenAccount = (await newAccountWithTokens({
    //   connection: provider.connection,
    //   MINT: nftMint,
    //   ADMIN_AUTH_KEYPAIR,
    //   userAccount: new anchor.web3.Account(),
    //   amount: 1
    // }))
  });

  it("deposit ", async () => {
    POSEIDON = await Poseidon.getInstance();

    KEYPAIR = new Account({
      poseidon: POSEIDON,
      seed: KEYPAIR_PRIVKEY.toString(),
    });

    const depositAmount =
      10_000 + (Math.floor(Math.random() * 1_000_000_000) % 1_100_000_000);
    const depositFeeAmount =
      10_000 + (Math.floor(Math.random() * 1_000_000_000) % 1_100_000_000);

    const tokenAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      ADMIN_AUTH_KEYPAIR,
      MINT,
      ADMIN_AUTH_KEYPAIR.publicKey,
    );
    await token.approve(
      provider.connection,
      ADMIN_AUTH_KEYPAIR,
      tokenAccount.address,
      Transaction.getSignerAuthorityPda(
        merkleTreeProgramId,
        new PublicKey(
          IDL_LIGHT_PSP2IN2OUT.constants[0].value.slice(
            1,
            IDL_LIGHT_PSP2IN2OUT.constants[0].value.length - 1,
          ),
        ),
      ), //delegate
      ADMIN_AUTH_KEYPAIR.publicKey, // owner
      depositAmount * 10,
      [ADMIN_AUTH_KEYPAIR],
    );
    const senderSpl = tokenAccount.address;

    const lightProvider = await Provider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
      confirmConfig,
    });

    deposit_utxo1 = new Utxo({
      poseidon: POSEIDON,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      publicKey: KEYPAIR.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });

    const txParams = new TransactionParameters({
      outputUtxos: [deposit_utxo1],
      eventMerkleTreePubkey: MerkleTreeConfig.getEventMerkleTreePda(),
      transactionMerkleTreePubkey:
        MerkleTreeConfig.getTransactionMerkleTreePda(),
      senderSpl: tokenAccount.address,
      senderSol: ADMIN_AUTH_KEYPAIR.publicKey,
      action: Action.SHIELD,
      poseidon: POSEIDON,
      verifierIdl: IDL_LIGHT_PSP2IN2OUT,
      account: KEYPAIR,
    });
    const { rootIndex, remainingAccounts } = await lightProvider.getRootIndex();
    const transaction = new Transaction({
      rootIndex,
      nextTransactionMerkleTree: remainingAccounts.nextTransactionMerkleTree,
      solMerkleTree: lightProvider.solMerkleTree!,
      params: txParams,
    });
    const instructions = await transaction.compileAndProve(
      lightProvider.poseidon,
      KEYPAIR,
    );
    console.log(transaction.params.accounts);

    // does one successful transaction
    try {
      await lightProvider.sendAndConfirmShieldedTransaction(instructions);
    } catch (e) {
      console.error(e);
    }
  });

  it("Update Merkle Tree Test", async () => {
    // Security Claims
    // CreateUpdateState
    // 1 leaves can only be inserted in the correct index order
    // 2 leaves cannot be inserted twice
    // 3 leaves are queued for a specific tree and can only be inserted in that tree
    // 4 lock is taken and cannot be taken again before expiry
    // 5 Merkle tree is registered
    //
    // Update
    // 6 signer is consistent
    // 7 is locked by update state account
    // 8 merkle tree is consistent
    //
    // Last Tx
    // 9 same leaves as in first tx are marked as inserted
    // 10 is in correct state
    // 11 is locked by update state account
    // 12 merkle tree is consistent
    // 13 signer is consistent

    const signer = ADMIN_AUTH_KEYPAIR;
    const transactionMerkleTreePubkey =
      MerkleTreeConfig.getTransactionMerkleTreePda();
    const eventMerkleTreePubkey = MerkleTreeConfig.getEventMerkleTreePda();

    await merkleTreeProgram.account.transactionMerkleTree.fetch(
      transactionMerkleTreePubkey,
    );
    let error;

    // fetch uninserted utxos from chain
    const leavesPdas = await SolMerkleTree.getUninsertedLeavesRelayer(
      transactionMerkleTreePubkey,
    );

    // build tree from chain
    const merkleTreeUpdateState = solana.PublicKey.findProgramAddressSync(
      [
        Buffer.from(new Uint8Array(signer.publicKey.toBytes())),
        anchor.utils.bytes.utf8.encode("storage"),
      ],
      merkleTreeProgram.programId,
    )[0];
    const connection = provider.connection;

    if (leavesPdas.length > 1) {
      // test leaves with higher starting index than merkle tree next index
      leavesPdas.reverse();
      try {
        await merkleTreeProgram.methods
          .initializeMerkleTreeUpdateState()
          .accounts({
            authority: signer.publicKey,
            merkleTreeUpdateState: merkleTreeUpdateState,
            systemProgram: DEFAULT_PROGRAMS.systemProgram,
            rent: DEFAULT_PROGRAMS.rent,
            transactionMerkleTree: transactionMerkleTreePubkey,
          })
          .remainingAccounts(leavesPdas)
          .preInstructions([
            solana.ComputeBudgetProgram.setComputeUnitLimit({
              units: 1_400_000,
            }),
          ])
          .signers([signer])
          .rpc(confirmConfig);
        console.log("success 0");
      } catch (e) {
        error = e;
      }
      assert(error.error.errorCode.code == "FirstLeavesPdaIncorrectIndex");

      leavesPdas.reverse();
      assert((await connection.getAccountInfo(merkleTreeUpdateState)) == null);

      console.log("Test property: 1");
      // Test property: 1
      // try with one leavespda of higher index
      try {
        await merkleTreeProgram.methods
          .initializeMerkleTreeUpdateState()
          .accounts({
            authority: signer.publicKey,
            merkleTreeUpdateState: merkleTreeUpdateState,
            systemProgram: SystemProgram.programId,
            rent: DEFAULT_PROGRAMS.rent,
            transactionMerkleTree: transactionMerkleTreePubkey,
          })
          .remainingAccounts([leavesPdas[1]])
          .preInstructions([
            solana.ComputeBudgetProgram.setComputeUnitLimit({
              units: 1_400_000,
            }),
          ])
          .signers([signer])
          .rpc(confirmConfig);
        console.log("success 1");
      } catch (e) {
        error = e;
      }
      assert(error.error.errorCode.code == "FirstLeavesPdaIncorrectIndex");

      assert((await connection.getAccountInfo(merkleTreeUpdateState)) == null);
    } else {
      console.log("pdas.length <=" + 1 + " skipping some tests");
    }

    const merkleTreeConfig = new MerkleTreeConfig({
      payer: ADMIN_AUTH_KEYPAIR,
      anchorProvider: provider,
    });

    // Check the next Merkle Tree indexes in Merkle Tree Authority. They should
    // be 1.
    const merkleTreeAuthorityAccountInfo =
      await merkleTreeConfig.getMerkleTreeAuthorityAccountInfo();
    assert(merkleTreeAuthorityAccountInfo.transactionMerkleTreeIndex.eq(BN_1));
    assert(merkleTreeAuthorityAccountInfo.eventMerkleTreeIndex.eq(BN_1));

    // Check if the previous Merkle Trees, before initializing the new ones,
    // are the newest ones. Check their indexes (0) as well.
    const transactionMerkleTreeAccountInfo =
      await merkleTreeConfig.getTransactionMerkleTreeAccountInfo(
        transactionMerkleTreePubkey,
      );
    assert.equal(transactionMerkleTreeAccountInfo.newest, 1);
    assert(transactionMerkleTreeAccountInfo.merkleTreeNr.eq(BN_0));
    const eventMerkleTreeAccountInfo =
      await merkleTreeConfig.getEventMerkleTreeAccountInfo(
        eventMerkleTreePubkey,
      );
    assert.equal(eventMerkleTreeAccountInfo.newest, 1);
    assert(eventMerkleTreeAccountInfo.merkleTreeNr.eq(BN_0));

    // Initialize new Merkle Trees.
    const newTransactionMerkleTreePubkey =
      MerkleTreeConfig.getTransactionMerkleTreePda(BN_1);
    const newEventMerkleTreePubkey =
      MerkleTreeConfig.getEventMerkleTreePda(BN_1);
    await merkleTreeConfig.initializeNewMerkleTrees();
    console.log("created new merkle trees");

    // Check if the previous Merkle Trees, after initializing the new ones,
    // aren't the newest ones anymore.
    const transactionMerkleTreeUpdatedAccountInfo =
      await merkleTreeConfig.getTransactionMerkleTreeAccountInfo(
        transactionMerkleTreePubkey,
      );
    assert.equal(transactionMerkleTreeUpdatedAccountInfo.newest, 0);
    const eventMerkleTreeUpdatedAccountInfo =
      await merkleTreeConfig.getEventMerkleTreeAccountInfo(
        eventMerkleTreePubkey,
      );
    assert.equal(eventMerkleTreeUpdatedAccountInfo.newest, 0);

    // Check if the new Merkle Trees are the newest ones. Check their indexes
    // (1) as well.
    const newTransactionMerkleTreeAccountInfo =
      await merkleTreeConfig.getTransactionMerkleTreeAccountInfo(
        newTransactionMerkleTreePubkey,
      );
    assert.equal(newTransactionMerkleTreeAccountInfo.newest, 1);
    assert(newTransactionMerkleTreeAccountInfo.merkleTreeNr.eq(BN_1));
    const newEventMerkleTreeAccountInfo =
      await merkleTreeConfig.getEventMerkleTreeAccountInfo(
        newEventMerkleTreePubkey,
      );
    assert.equal(newEventMerkleTreeAccountInfo.newest, 1);
    assert(newEventMerkleTreeAccountInfo.merkleTreeNr.eq(BN_1));

    // Check the next Merkle Tree indexes in MerkleTreeAuthority. They should
    // be 2.
    const merkleTreeAuthorityUpdatedAccountInfo =
      await merkleTreeConfig.getMerkleTreeAuthorityAccountInfo();
    assert(
      merkleTreeAuthorityUpdatedAccountInfo.transactionMerkleTreeIndex.eq(BN_2),
    );
    assert(merkleTreeAuthorityUpdatedAccountInfo.eventMerkleTreeIndex.eq(BN_2));

    // Test property: 3
    // try with different Merkle tree than leaves are queued for
    // index might be broken it is wasn't set to mut didn't update
    try {
      await merkleTreeProgram.methods
        .initializeMerkleTreeUpdateState()
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          systemProgram: SystemProgram.programId,
          rent: DEFAULT_PROGRAMS.rent,
          transactionMerkleTree: newTransactionMerkleTreePubkey,
        })
        .remainingAccounts(leavesPdas)
        .preInstructions([
          solana.ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
        ])
        .signers([signer])
        .rpc(confirmConfig);
      console.log("success 3");
    } catch (e) {
      console.log(e);
      error = e;
    }
    assert(error.error.errorCode.code == "LeavesOfWrongTree");
    assert((await connection.getAccountInfo(merkleTreeUpdateState)) == null);
    error = undefined;

    await merkleTreeConfig.updateLockDuration(0);
    // correct
    try {
      await merkleTreeProgram.methods
        .initializeMerkleTreeUpdateState()
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          systemProgram: SystemProgram.programId,
          rent: DEFAULT_PROGRAMS.rent,
          transactionMerkleTree: transactionMerkleTreePubkey,
        })
        .remainingAccounts([leavesPdas[0]])
        .preInstructions([
          solana.ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
        ])
        .signers([signer])
        .rpc(confirmConfig);
    } catch (e) {
      error = e;
      console.log(error);
    }
    // should not be an error
    assert(error === undefined);
    console.log("created update state ", merkleTreeUpdateState.toBase58());

    assert((await connection.getAccountInfo(merkleTreeUpdateState)) != null);

    await checkMerkleTreeUpdateStateCreated({
      connection: connection,
      merkleTreeUpdateState,
      transactionMerkleTree: transactionMerkleTreePubkey,
      relayer: signer.publicKey,
      leavesPdas: [leavesPdas[0]],
      current_instruction_index: 1,
      merkleTreeProgram,
    });

    // close merkletreeupdatestate
    try {
      console.log("closeMerkleTreeUpdateState 1");
      await closeMerkleTreeUpdateState(merkleTreeProgram, signer, connection);
      assert((await connection.getAccountInfo(merkleTreeUpdateState)) === null);
    } catch (e) {
      error = e;
      console.log(error);
      throw e;
    }

    // init again
    try {
      await merkleTreeProgram.methods
        .initializeMerkleTreeUpdateState()
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          systemProgram: SystemProgram.programId,
          rent: DEFAULT_PROGRAMS.rent,
          transactionMerkleTree: transactionMerkleTreePubkey,
        })
        .remainingAccounts([leavesPdas[0]])
        .preInstructions([
          solana.ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
        ])
        .signers([signer])
        .rpc(confirmConfig);
    } catch (e) {
      error = e;
      console.log(error);
    }

    // should not be an error
    assert(error === undefined);
    console.log("created update state ", merkleTreeUpdateState.toBase58());

    assert((await connection.getAccountInfo(merkleTreeUpdateState)) != null);

    await checkMerkleTreeUpdateStateCreated({
      connection: connection,
      merkleTreeUpdateState,
      transactionMerkleTree: transactionMerkleTreePubkey,
      relayer: signer.publicKey,
      leavesPdas: [leavesPdas[0]],
      current_instruction_index: 1,
      merkleTreeProgram,
    });

    console.log("executeMerkleTreeUpdateTransactions 10");

    await executeMerkleTreeUpdateTransactions({
      signer,
      merkleTreeProgram,
      transactionMerkleTree: transactionMerkleTreePubkey,
      connection: provider.connection,
      merkleTreeUpdateState,
      numberOfTransactions: 10,
      interrupt: true,
    });
    console.log("checkMerkleTreeUpdateStateCreated 22");

    await checkMerkleTreeUpdateStateCreated({
      connection: connection,
      merkleTreeUpdateState,
      transactionMerkleTree: transactionMerkleTreePubkey,
      relayer: signer.publicKey,
      leavesPdas: [leavesPdas[0]],
      current_instruction_index: 22, // 22 because one tx executes two instructions, it started out in ix index 1 and increments at the end of a tx
      merkleTreeProgram,
    });

    // Test property: 6
    // trying to use merkleTreeUpdateState with different signer

    const maliciousSigner = await newAccountWithLamports(provider.connection);
    console.log("maliciousSigner: ", maliciousSigner.publicKey.toBase58());

    const maliciousMerkleTreeUpdateState =
      solana.PublicKey.findProgramAddressSync(
        [
          Buffer.from(new Uint8Array(maliciousSigner.publicKey.toBytes())),
          anchor.utils.bytes.utf8.encode("storage"),
        ],
        merkleTreeProgram.programId,
      )[0];

    try {
      await executeMerkleTreeUpdateTransactions({
        signer: maliciousSigner,
        merkleTreeProgram,
        transactionMerkleTree: transactionMerkleTreePubkey,
        connection: provider.connection,
        merkleTreeUpdateState,
        numberOfTransactions: 1,
        interrupt: true,
      });
    } catch (err) {
      error = err;
    }
    assert(
      error.logs.includes(
        "Program log: AnchorError caused by account: authority. Error Code: InvalidAuthority. Error Number: 6016. Error Message: InvalidAuthority.",
      ),
    );
    await merkleTreeConfig.updateLockDuration(10);

    // Test property: 4
    // try to take lock
    try {
      await merkleTreeProgram.methods
        .initializeMerkleTreeUpdateState()
        .accounts({
          authority: maliciousSigner.publicKey,
          merkleTreeUpdateState: maliciousMerkleTreeUpdateState,
          systemProgram: SystemProgram.programId,
          rent: DEFAULT_PROGRAMS.rent,
          transactionMerkleTree: transactionMerkleTreePubkey,
        })
        .remainingAccounts([leavesPdas[0]])
        .signers([maliciousSigner])
        .rpc(confirmConfig);
    } catch (e) {
      error = e;
      console.log(e);
    }
    assert(error.error.errorCode.code == "ContractStillLocked");

    // Test property: 10
    // try insert root before completing update transaction
    try {
      await merkleTreeProgram.methods
        .insertRootMerkleTree(new anchor.BN(254))
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          transactionMerkleTree: transactionMerkleTreePubkey,
          logWrapper: SPL_NOOP_ADDRESS,
        })
        .signers([signer])
        .rpc(confirmConfig);
    } catch (e) {
      error = e;
    }
    console.log(error);

    assert(error.error.errorCode.code == "MerkleTreeUpdateNotInRootInsert");

    // sending additional tx to finish the merkle tree update
    await executeMerkleTreeUpdateTransactions({
      signer,
      merkleTreeProgram,
      transactionMerkleTree: transactionMerkleTreePubkey,
      connection: provider.connection,
      merkleTreeUpdateState,
      numberOfTransactions: 50,
    });

    await checkMerkleTreeUpdateStateCreated({
      connection: connection,
      merkleTreeUpdateState,
      transactionMerkleTree: transactionMerkleTreePubkey,
      relayer: signer.publicKey,
      leavesPdas: [leavesPdas[0]],
      current_instruction_index: 56,
      merkleTreeProgram,
    });

    // Test property: 11
    // final tx to insert root different UNREGISTERED_MERKLE_TREE
    try {
      console.log("final tx to insert root into different_merkle_tree");
      await merkleTreeProgram.methods
        .insertRootMerkleTree(new anchor.BN(254))
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          transactionMerkleTree: newTransactionMerkleTreePubkey,
          logWrapper: SPL_NOOP_ADDRESS,
        })
        .signers([signer])
        .rpc(confirmConfig);
    } catch (e) {
      error = e;
    }
    assert(error.error.errorCode.code == "ContractStillLocked");

    // Test property: 13
    // final tx to insert root different signer
    try {
      await merkleTreeProgram.methods
        .insertRootMerkleTree(new anchor.BN(254))
        .accounts({
          authority: maliciousSigner.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          transactionMerkleTree: transactionMerkleTreePubkey,
          logWrapper: SPL_NOOP_ADDRESS,
        })
        .signers([maliciousSigner])
        .rpc(confirmConfig);
    } catch (e) {
      error = e;
    }
    assert(error.error.errorCode.code == "InvalidAuthority");

    const merkleTreeAccountPrior =
      await merkleTreeProgram.account.transactionMerkleTree.fetch(
        transactionMerkleTreePubkey,
      );

    const indexedTransactions = await RELAYER.getIndexedTransactions(
      provider!.connection,
    );

    const merkleTree = await SolMerkleTree.build({
      pubkey: transactionMerkleTreePubkey,
      poseidon: POSEIDON,
      indexedTransactions,
      provider: provider,
    });

    // insert correctly
    await merkleTreeProgram.methods
      .insertRootMerkleTree(new anchor.BN(254))
      .accounts({
        authority: signer.publicKey,
        merkleTreeUpdateState: merkleTreeUpdateState,
        transactionMerkleTree: transactionMerkleTreePubkey,
        logWrapper: SPL_NOOP_ADDRESS,
      })
      .signers([signer])
      .rpc(confirmConfig);
    console.log("merkleTreeUpdateState ", merkleTreeUpdateState);
    console.log("merkleTreeAccountPrior ", merkleTreeAccountPrior);
    console.log("leavesPdas[0] ", leavesPdas[0]);
    console.log("merkleTree ", merkleTree);
    console.log("merkle_tree_pubkey ", transactionMerkleTreePubkey);

    await checkMerkleTreeBatchUpdateSuccess({
      connection: provider.connection,
      merkleTreeUpdateState: merkleTreeUpdateState,
      merkleTreeAccountPrior,
      numberOfLeaves: 2,
      leavesPdas: [leavesPdas[0]],
      transactionMerkleTree: transactionMerkleTreePubkey,
      merkleTreeProgram,
    });

    console.log("Test property: 2");

    // Test property: 2
    // try to reinsert leavesPdas[0]
    try {
      await merkleTreeProgram.methods
        .initializeMerkleTreeUpdateState()
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          systemProgram: SystemProgram.programId,
          rent: DEFAULT_PROGRAMS.rent,
          transactionMerkleTree: transactionMerkleTreePubkey,
        })
        .remainingAccounts([leavesPdas[0]])
        .preInstructions([
          solana.ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
        ])
        .signers([signer])
        .rpc(confirmConfig);
    } catch (e) {
      error = e;
    }
    assert(error.error.errorCode.code == "LeafAlreadyInserted");
  });

  it("Switch to a new Merkle tree", async () => {
    const shieldAmount = new anchor.BN(1_000_000);
    const shieldFeeAmount = new anchor.BN(1_000_000);

    const tokenAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      ADMIN_AUTH_KEYPAIR,
      MINT,
      ADMIN_AUTH_KEYPAIR.publicKey,
    );
    const senderSpl = tokenAccount.address;
    const lightProvider = await Provider.init({
      wallet: ADMIN_AUTH_KEYPAIR,
      relayer: RELAYER,
      confirmConfig,
    });

    const shieldUtxo = new Utxo({
      poseidon: POSEIDON,
      assets: [FEE_ASSET, MINT],
      amounts: [shieldFeeAmount, shieldAmount],
      publicKey: KEYPAIR.pubkey,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });

    const newTransactionMerkleTreePubkey =
      MerkleTreeConfig.getTransactionMerkleTreePda(BN_1);
    const newEventMerkleTreePubkey =
      MerkleTreeConfig.getEventMerkleTreePda(BN_1);

    const txParams = new TransactionParameters({
      outputUtxos: [shieldUtxo],
      eventMerkleTreePubkey: MerkleTreeConfig.getEventMerkleTreePda(),
      transactionMerkleTreePubkey:
        MerkleTreeConfig.getTransactionMerkleTreePda(),
      senderSpl,
      senderSol: ADMIN_AUTH_KEYPAIR.publicKey,
      action: Action.SHIELD,
      poseidon: POSEIDON,
      verifierIdl: IDL_LIGHT_PSP2IN2OUT,
      account: KEYPAIR,
    });

    const { rootIndex, remainingAccounts } = await lightProvider.getRootIndex();
    const transaction = new Transaction({
      rootIndex,
      nextTransactionMerkleTree: remainingAccounts.nextTransactionMerkleTree,
      solMerkleTree: lightProvider.solMerkleTree!,
      params: txParams,
    });
    transaction.remainingAccounts!.nextTransactionMerkleTree = {
      isSigner: false,
      isWritable: true,
      pubkey: newTransactionMerkleTreePubkey,
    };
    transaction.remainingAccounts!.nextEventMerkleTree = {
      isSigner: false,
      isWritable: true,
      pubkey: newEventMerkleTreePubkey,
    };

    const instructions = await transaction.compileAndProve(
      lightProvider.poseidon,
      KEYPAIR,
    );
    await lightProvider.sendAndConfirmTransaction(instructions);

    const leavesPdas = await SolMerkleTree.getUninsertedLeavesRelayer(
      newTransactionMerkleTreePubkey,
    );

    executeUpdateMerkleTreeTransactions({
      connection: provider.connection,
      signer: ADMIN_AUTH_KEYPAIR,
      merkleTreeProgram,
      leavesPdas,
      transactionMerkleTree: newTransactionMerkleTreePubkey,
    });
  });
});
