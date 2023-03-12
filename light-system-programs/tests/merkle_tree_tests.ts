import * as anchor from "@coral-xyz/anchor";
import { SystemProgram, Keypair as SolanaKeypair } from "@solana/web3.js";
const solana = require("@solana/web3.js");
import _ from "lodash";
import { assert, expect } from "chai";
const token = require("@solana/spl-token");
let circomlibjs = require("circomlibjs");

import {
  Transaction,
  VerifierZero,
  Utxo,
  createMintWrapper,
  initLookUpTableFromFile,
  MerkleTreeProgram,
  merkleTreeProgramId,
  IDL_MERKLE_TREE_PROGRAM,
  MERKLE_TREE_KEY,
  ADMIN_AUTH_KEYPAIR,
  MINT,
  KEYPAIR_PRIVKEY,
  USER_TOKEN_ACCOUNT,
  createTestAccounts,
  userTokenAccount,
  FEE_ASSET,
  confirmConfig,
  TransactionParameters,
  SolMerkleTree,
  VerifierProgramZero,
  verifierProgramZeroProgramId,
  MerkleTreeConfig,
  DEFAULT_PROGRAMS,
  checkMerkleTreeUpdateStateCreated,
  executeMerkleTreeUpdateTransactions,
  newAccountWithLamports,
  checkMerkleTreeBatchUpdateSuccess,
  POOL_TYPE,
  IDL_VERIFIER_PROGRAM_ZERO,
  Account,
  Provider,
} from "light-sdk";
import { SPL_NOOP_ADDRESS } from "@solana/spl-account-compression";

var LOOK_UP_TABLE, POSEIDON, KEYPAIR, deposit_utxo1;

console.log = () => {};
describe("Merkle Tree Tests", () => {
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
  // Configure the client to use the local cluster.
  var provider = anchor.AnchorProvider.local(
    "http://127.0.0.1:8899",
    confirmConfig,
  );
  process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";

  anchor.setProvider(provider);
  const merkleTreeProgram: anchor.Program<MerkleTreeProgram> =
    new anchor.Program(IDL_MERKLE_TREE_PROGRAM, merkleTreeProgramId);

  var INVALID_MERKLE_TREE_AUTHORITY_PDA, INVALID_SIGNER;
  before(async () => {
    await createTestAccounts(provider.connection, userTokenAccount);
    LOOK_UP_TABLE = await initLookUpTableFromFile(provider);
    // await setUpMerkleTree(provider);

    var merkleTreeAccountInfoInit = await provider.connection.getAccountInfo(
      MERKLE_TREE_KEY,
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
  });

  const test = async (fn: any, obj: any, error: string, args?: any) => {
    fn = fn.bind(obj);
    try {
      if (args) {
        expect(await fn(args)).throw();
        s;
      } else {
        expect(await fn()).throw();
      }
    } catch (e) {
      console.log(e);
      assert.isTrue(e.logs.includes(error));
    }
  };

  it.skip("Build Merkle Tree from account compression", async () => {
    const poseidon = await circomlibjs.buildPoseidonOpt();
    let merkleTree = await SolMerkleTree.build({
      pubkey: MERKLE_TREE_KEY,
      poseidon,
    });

    let newTree = await merkleTreeProgram.account.merkleTree.fetch(
      MERKLE_TREE_KEY,
    );
    assert.equal(
      merkleTree.merkleTree.root(),
      new anchor.BN(
        newTree.roots[newTree.currentRootIndex.toNumber()],
        32,
        "le",
      ),
    );
  });

  it("Initialize Merkle Tree Test", async () => {
    const verifierProgramZero = new anchor.Program(
      IDL_VERIFIER_PROGRAM_ZERO,
      verifierProgramZeroProgramId,
    );
    // const verifierProgramOne = new anchor.Program(VerifierProgramOne, verifierProgramOneProgramId);

    // Security Claims
    // Init authority pda
    // - can only be inited by a hardcoded pubkey
    // Update authority pda
    // - can only be invoked by current authority

    var merkleTreeAccountInfoInit = await provider.connection.getAccountInfo(
      MERKLE_TREE_KEY,
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
    let merkleTreeConfig = new MerkleTreeConfig({
      merkleTreePubkey: MERKLE_TREE_KEY,
      payer: ADMIN_AUTH_KEYPAIR,
      connection: provider.connection,
    });
    await merkleTreeConfig.getMerkleTreeAuthorityPda();

    let error;

    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
    try {
      await merkleTreeConfig.initMerkleTreeAuthority();
    } catch (e) {
      error = e;
    }
    await merkleTreeConfig.getMerkleTreeAuthorityPda();
    console.log(error);

    assert.isTrue(
      error.logs.includes(
        // "Program log: AnchorError caused by account: merkle_tree_authority_pda. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated."
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
    console.log(error);

    assert.isTrue(
      error.logs.includes(
        "Program log: Instruction: InitializeMerkleTreeAuthority",
      ),
    );
    error = undefined;

    // initing real mt authority
    await merkleTreeConfig.initMerkleTreeAuthority();
    await merkleTreeConfig.initializeNewMerkleTree();

    let newAuthority = new anchor.web3.Account();
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
    assert.equal(error.error.errorMessage, "InvalidAuthority");
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
    assert.equal(
      error.error.errorMessage,
      "The program expected this account to be already initialized",
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
      await merkleTreeConfig.registerVerifier(verifierProgramZero.programId);
    } catch (e) {
      error = e;
    }
    console.log(error);

    assert.equal(error.error.errorMessage, "InvalidAuthority");
    error = undefined;
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;

    // invalid pda
    let tmp = merkleTreeConfig.registeredVerifierPdas[0].registeredVerifierPda;
    merkleTreeConfig.registeredVerifierPdas[0].registeredVerifierPda =
      INVALID_SIGNER.publicKey;
    try {
      await merkleTreeConfig.registerVerifier(verifierProgramZero.programId);
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

    // update merkle tree with invalid signer
    // merkleTreeConfig.payer = INVALID_SIGNER;
    // try {
    //   await merkleTreeConfig.enableNfts(true);
    // } catch (e) {
    //   error = e;
    // }
    // assert.equal(error.error.errorMessage, "InvalidAuthority");
    // error = undefined;
    // merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;

    // // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
    // merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
    // try {
    //   await merkleTreeConfig.enableNfts(true);
    // } catch (e) {
    //   error = e;
    // }
    // await merkleTreeConfig.getMerkleTreeAuthorityPda();
    // assert.equal(
    //   error.error.errorMessage,
    //   "The program expected this account to be already initialized"
    // );
    // error = undefined;

    // await merkleTreeConfig.enableNfts(true);

    let merkleTreeAuthority =
      await merkleTreeProgram.account.merkleTreeAuthority.fetch(
        merkleTreeConfig.merkleTreeAuthorityPda,
      );
    // assert.equal(merkleTreeAuthority.enableNfts, true);
    // await merkleTreeConfig.enableNfts(false);
    // merkleTreeAuthority =
    //   await merkleTreeProgram.account.merkleTreeAuthority.fetch(
    //     merkleTreeConfig.merkleTreeAuthorityPda
    //   );
    // assert.equal(merkleTreeAuthority.enableNfts, false);

    // update lock duration with invalid signer

    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.updateLockDuration(123);
    } catch (e) {
      error = e;
    }

    assert.equal(error.error.errorMessage, "InvalidAuthority");
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
    assert.equal(
      error.error.errorMessage,
      "The program expected this account to be already initialized",
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

    assert.equal(error.error.errorMessage, "InvalidAuthority");
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

    assert.equal(
      error.error.errorMessage,
      "The program expected this account to be already initialized",
    );
    error = undefined;

    await merkleTreeConfig.enablePermissionlessSplTokens(true);

    merkleTreeAuthority =
      await merkleTreeProgram.account.merkleTreeAuthority.fetch(
        merkleTreeConfig.merkleTreeAuthorityPda,
      );
    assert.equal(merkleTreeAuthority.enablePermissionlessSplTokens, true);
    await merkleTreeConfig.enablePermissionlessSplTokens(false);
    merkleTreeAuthority =
      await merkleTreeProgram.account.merkleTreeAuthority.fetch(
        merkleTreeConfig.merkleTreeAuthorityPda,
      );
    assert.equal(merkleTreeAuthority.enablePermissionlessSplTokens, false);

    // update merkle tree with invalid signer
    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.registerPoolType(new Uint8Array(32).fill(0));
    } catch (e) {
      error = e;
    }

    assert.equal(error.error.errorMessage, "InvalidAuthority");
    error = undefined;
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;

    // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
    try {
      await merkleTreeConfig.registerPoolType(new Uint8Array(32).fill(0));
    } catch (e) {
      error = e;
    }
    await merkleTreeConfig.getMerkleTreeAuthorityPda();

    assert.equal(
      error.error.errorMessage,
      "The program expected this account to be already initialized",
    );
    error = undefined;

    await merkleTreeConfig.registerPoolType(new Uint8Array(32).fill(0));

    let registeredPoolTypePdaAccount =
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
      await merkleTreeConfig.registerSolPool(new Uint8Array(32).fill(0));
    } catch (e) {
      error = e;
    }
    console.log(error);

    assert.equal(error.error.errorMessage, "InvalidAuthority");
    error = undefined;
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;

    // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
    try {
      await merkleTreeConfig.registerSolPool(new Uint8Array(32).fill(0));
    } catch (e) {
      error = e;
    }
    await merkleTreeConfig.getMerkleTreeAuthorityPda();
    console.log("error ", error);

    assert.equal(
      error.error.errorMessage,
      "The program expected this account to be already initialized",
    );
    error = undefined;

    // valid
    await merkleTreeConfig.registerSolPool(new Uint8Array(32).fill(0));
    console.log("merkleTreeConfig ", merkleTreeConfig);

    let registeredSolPdaAccount =
      await merkleTreeProgram.account.registeredAssetPool.fetch(
        MerkleTreeConfig.getSolPoolPda(merkleTreeProgramId).pda,
      );
    assert.equal(
      registeredSolPdaAccount.poolType.toString(),
      new Uint8Array(32).fill(0).toString(),
    );
    assert.equal(registeredSolPdaAccount.index, 0);
    assert.equal(
      registeredSolPdaAccount.assetPoolPubkey.toBase58(),
      MerkleTreeConfig.getSolPoolPda(merkleTreeProgramId).pda.toBase58(),
    );

    let mint = await createMintWrapper({
      authorityKeypair: ADMIN_AUTH_KEYPAIR,
      connection: provider.connection,
    });

    // update merkle tree with invalid signer
    merkleTreeConfig.payer = INVALID_SIGNER;
    try {
      await merkleTreeConfig.registerSplPool(new Uint8Array(32).fill(0), mint);
    } catch (e) {
      error = e;
    }

    assert.equal(error.error.errorMessage, "InvalidAuthority");
    error = undefined;
    merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;

    // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
    merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
    try {
      await merkleTreeConfig.registerSplPool(new Uint8Array(32).fill(0), mint);
    } catch (e) {
      error = e;
    }
    await merkleTreeConfig.getMerkleTreeAuthorityPda();

    assert.equal(
      error.error.errorMessage,
      "The program expected this account to be already initialized",
    );
    error = undefined;

    // valid
    await merkleTreeConfig.registerSplPool(new Uint8Array(32).fill(0), mint);
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

    let merkleTreeAuthority1 =
      await merkleTreeProgram.account.merkleTreeAuthority.fetch(
        merkleTreeConfig.merkleTreeAuthorityPda,
      );
    console.log(merkleTreeAuthority1);
    assert.equal(merkleTreeAuthority1.registeredAssetIndex.toString(), "2");
    await merkleTreeConfig.registerVerifier(verifierProgramZero.programId);
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
    // await createTestAccounts(provider.connection);
    // LOOK_UP_TABLE = await initLookUpTableFromFile(provider);
    // await setUpMerkleTree(provider);

    POSEIDON = await circomlibjs.buildPoseidonOpt();

    KEYPAIR = new Account({
      poseidon: POSEIDON,
      seed: KEYPAIR_PRIVKEY.toString(),
    });

    var depositAmount =
      10_000 + (Math.floor(Math.random() * 1_000_000_000) % 1_100_000_000);
    var depositFeeAmount =
      10_000 + (Math.floor(Math.random() * 1_000_000_000) % 1_100_000_000);

    await token.approve(
      provider.connection,
      ADMIN_AUTH_KEYPAIR,
      userTokenAccount,
      Transaction.getSignerAuthorityPda(
        merkleTreeProgramId,
        new VerifierZero().verifierProgram.programId,
      ), //delegate
      USER_TOKEN_ACCOUNT, // owner
      depositAmount * 10,
      [USER_TOKEN_ACCOUNT],
    );

    let lightProvider = await Provider.native(ADMIN_AUTH_KEYPAIR);

    var transaction = new Transaction({
      provider: lightProvider,
    });

    deposit_utxo1 = new Utxo({
      poseidon: POSEIDON,
      assets: [FEE_ASSET, MINT],
      amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
      account: KEYPAIR,
    });

    let txParams = new TransactionParameters({
      outputUtxos: [deposit_utxo1],
      merkleTreePubkey: MERKLE_TREE_KEY,
      sender: userTokenAccount,
      senderFee: ADMIN_AUTH_KEYPAIR.publicKey,
      verifier: new VerifierZero(),
    });
    await transaction.compileAndProve(txParams);
    console.log(transaction.params.accounts);

    // does one successful transaction
    await transaction.sendAndConfirmTransaction();
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

    let mtFetched = await merkleTreeProgram.account.merkleTree.fetch(
      MERKLE_TREE_KEY,
    );
    let error;

    // fetch uninserted utxos from chain
    let leavesPdas = await SolMerkleTree.getUninsertedLeavesRelayer(
      MERKLE_TREE_KEY,
    );

    let poseidon = await circomlibjs.buildPoseidonOpt();
    // build tree from chain
    // let merkleTree = await SolMerkleTree.build({pubkey: MERKLE_TREE_KEY, poseidon: POSEIDON})

    let merkleTreeUpdateState = solana.PublicKey.findProgramAddressSync(
      [
        Buffer.from(new Uint8Array(signer.publicKey.toBytes())),
        anchor.utils.bytes.utf8.encode("storage"),
      ],
      merkleTreeProgram.programId,
    )[0];
    let merkle_tree_pubkey = MERKLE_TREE_KEY;
    let connection = provider.connection;

    if (leavesPdas.length > 1) {
      // test leaves with higher starting index than merkle tree next index
      leavesPdas.reverse();
      try {
        const tx1 = await merkleTreeProgram.methods
          .initializeMerkleTreeUpdateState()
          .accounts({
            authority: signer.publicKey,
            merkleTreeUpdateState: merkleTreeUpdateState,
            systemProgram: DEFAULT_PROGRAMS.systemProgram,
            rent: DEFAULT_PROGRAMS.rent,
            merkleTree: merkle_tree_pubkey,
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
        const tx1 = await merkleTreeProgram.methods
          .initializeMerkleTreeUpdateState()
          .accounts({
            authority: signer.publicKey,
            merkleTreeUpdateState: merkleTreeUpdateState,
            systemProgram: SystemProgram.programId,
            rent: DEFAULT_PROGRAMS.rent,
            merkleTree: merkle_tree_pubkey,
          })
          .remainingAccounts(leavesPdas[1])
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

    // Test property: 3
    // try with different Merkle tree than leaves are queued for
    // index might be broken it is wasn't set to mut didn't update
    let merkleTreeConfig = new MerkleTreeConfig({
      merkleTreePubkey: MERKLE_TREE_KEY,
      payer: ADMIN_AUTH_KEYPAIR,
      connection: provider.connection,
    });
    let different_merkle_tree = solana.PublicKey.findProgramAddressSync(
      [
        merkleTreeProgram.programId.toBuffer(),
        new anchor.BN(1).toArray("le", 8),
      ],
      merkleTreeProgram.programId,
    )[0];
    if ((await connection.getAccountInfo(different_merkle_tree)) == null) {
      await merkleTreeConfig.initializeNewMerkleTree(different_merkle_tree);
      console.log("created new merkle tree");
    }

    try {
      const tx1 = await merkleTreeProgram.methods
        .initializeMerkleTreeUpdateState()
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          systemProgram: SystemProgram.programId,
          rent: DEFAULT_PROGRAMS.rent,
          merkleTree: different_merkle_tree,
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

    // correct
    try {
      const tx1 = await merkleTreeProgram.methods
        .initializeMerkleTreeUpdateState()
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          systemProgram: SystemProgram.programId,
          rent: DEFAULT_PROGRAMS.rent,
          merkleTree: merkle_tree_pubkey,
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
      MerkleTree: merkle_tree_pubkey,
      relayer: signer.publicKey,
      leavesPdas: [leavesPdas[0]],
      current_instruction_index: 1,
      merkleTreeProgram,
    });
    console.log("executeMerkleTreeUpdateTransactions 10");

    await executeMerkleTreeUpdateTransactions({
      signer,
      merkleTreeProgram,
      merkle_tree_pubkey,
      connection: provider.connection,
      merkleTreeUpdateState,
      numberOfTransactions: 10,
    });
    console.log("checkMerkleTreeUpdateStateCreated 22");

    await checkMerkleTreeUpdateStateCreated({
      connection: connection,
      merkleTreeUpdateState,
      MerkleTree: merkle_tree_pubkey,
      relayer: signer.publicKey,
      leavesPdas: [leavesPdas[0]],
      current_instruction_index: 22, // 22 becaue one tx executes two instructions, it started out in ix index 1 and increments at the end of a tx
      merkleTreeProgram,
    });

    // Test property: 6
    // trying to use merkleTreeUpdateState with different signer

    let maliciousSigner = await newAccountWithLamports(provider.connection);
    console.log("maliciousSigner: ", maliciousSigner.publicKey.toBase58());

    let maliciousMerkleTreeUpdateState =
      solana.PublicKey.findProgramAddressSync(
        [
          Buffer.from(new Uint8Array(maliciousSigner.publicKey.toBytes())),
          anchor.utils.bytes.utf8.encode("storage"),
        ],
        merkleTreeProgram.programId,
      )[0];
    let s = false;
    try {
      await executeMerkleTreeUpdateTransactions({
        signer: maliciousSigner,
        merkleTreeProgram,
        merkle_tree_pubkey,
        connection: provider.connection,
        merkleTreeUpdateState,
        numberOfTransactions: 1,
      });
    } catch (err) {
      error = err;
    }
    assert(
      error.logs.includes(
        "Program log: AnchorError caused by account: authority. Error Code: InvalidAuthority. Error Number: 6016. Error Message: InvalidAuthority.",
      ),
    );

    // Test property: 4
    // try to take lock
    try {
      const tx1 = await merkleTreeProgram.methods
        .initializeMerkleTreeUpdateState()
        .accounts({
          authority: maliciousSigner.publicKey,
          merkleTreeUpdateState: maliciousMerkleTreeUpdateState,
          systemProgram: SystemProgram.programId,
          rent: DEFAULT_PROGRAMS.rent,
          merkleTree: merkle_tree_pubkey,
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
          merkleTree: merkle_tree_pubkey,
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
      merkle_tree_pubkey,
      connection: provider.connection,
      merkleTreeUpdateState,
      numberOfTransactions: 50,
    });

    await checkMerkleTreeUpdateStateCreated({
      connection: connection,
      merkleTreeUpdateState,
      MerkleTree: merkle_tree_pubkey,
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
          merkleTree: different_merkle_tree,
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
          merkleTree: merkle_tree_pubkey,
          logWrapper: SPL_NOOP_ADDRESS,
        })
        .signers([maliciousSigner])
        .rpc(confirmConfig);
    } catch (e) {
      error = e;
    }
    assert(error.error.errorCode.code == "InvalidAuthority");

    var merkleTreeAccountPrior =
      await merkleTreeProgram.account.merkleTree.fetch(merkle_tree_pubkey);

    let merkleTree = await SolMerkleTree.build({
      pubkey: MERKLE_TREE_KEY,
      poseidon: POSEIDON,
    });

    // insert correctly
    await merkleTreeProgram.methods
      .insertRootMerkleTree(new anchor.BN(254))
      .accounts({
        authority: signer.publicKey,
        merkleTreeUpdateState: merkleTreeUpdateState,
        merkleTree: merkle_tree_pubkey,
        logWrapper: SPL_NOOP_ADDRESS,
      })
      .signers([signer])
      .rpc(confirmConfig);
    console.log("merkleTreeUpdateState ", merkleTreeUpdateState);
    console.log("merkleTreeAccountPrior ", merkleTreeAccountPrior);
    console.log("leavesPdas[0] ", leavesPdas[0]);
    console.log("merkleTree ", merkleTree);
    console.log("merkle_tree_pubkey ", merkle_tree_pubkey);

    await checkMerkleTreeBatchUpdateSuccess({
      connection: provider.connection,
      merkleTreeUpdateState: merkleTreeUpdateState,
      merkleTreeAccountPrior,
      numberOfLeaves: 2,
      leavesPdas: [leavesPdas[0]],
      merkle_tree_pubkey: merkle_tree_pubkey,
      merkleTreeProgram,
    });

    console.log("Test property: 2");

    // Test property: 2
    // try to reinsert leavesPdas[0]
    try {
      const tx1 = await merkleTreeProgram.methods
        .initializeMerkleTreeUpdateState()
        .accounts({
          authority: signer.publicKey,
          merkleTreeUpdateState: merkleTreeUpdateState,
          systemProgram: SystemProgram.programId,
          rent: DEFAULT_PROGRAMS.rent,
          merkleTree: merkle_tree_pubkey,
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
});
