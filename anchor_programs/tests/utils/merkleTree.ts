import * as anchor from "@project-serum/anchor";
const solana = require("@solana/web3.js");
import { MerkleTreeProgram } from "../../target/types/merkle_tree_program";
import { assert, expect } from "chai";
const token = require('@solana/spl-token')
const light = require('../../light-protocol-sdk');
import {Connection, PublicKey, Keypair} from "@solana/web3.js";

import {
  DEFAULT_PROGRAMS
} from "./constants";

export class MerkleTreeConfig {
  constructor({
    merkleTreePubkey,
    payer,
    connection
  }) {
      this.merkleTreePubkey =merkleTreePubkey;
      this.payer = payer;
      this.merkleTreeProgram = anchor.workspace.MerkleTreeProgram as Program<MerkleTreeProgram>;
      // TODO: reorg pool pdas, have one object per pool type and then an array with registered pools of this type
      this.poolPdas = [];
      this.poolTypes = [];
      this.registeredVerifierPdas = [];
      this.connection = connection;
    }

    async getPreInsertedLeavesIndex() {
        this.preInsertedLeavesIndex = (await solana.PublicKey.findProgramAddress(
            [this.merkleTreePubkey.toBuffer()],
            this.merkleTreeProgram.programId))[0];
        return this.preInsertedLeavesIndex;
    }

    async initializeNewMerkleTree(merkleTreePubkey) {
      if (merkleTreePubkey) {
        this.merkleTreePubkey = merkleTreePubkey;
      }
      await this.getPreInsertedLeavesIndex();
      await this.getMerkleTreeAuthorityPda();
      const tx = await this.merkleTreeProgram.methods.initializeNewMerkleTree(
        new anchor.BN("50")
      ).accounts({
        authority: this.payer.publicKey,
        merkleTree: this.merkleTreePubkey,
        preInsertedLeavesIndex: this.preInsertedLeavesIndex,
        systemProgram: DEFAULT_PROGRAMS.systemProgram,
        rent: DEFAULT_PROGRAMS.rent,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda
      })
      .signers([this.payer])
      .rpc({commitment: "finalized", preflightCommitment: 'finalized',});

      await this.checkMerkleTreeIsInitialized()
      await this.checkPreInsertedLeavesIndexIsInitialized()
      return tx;
    }

    async checkMerkleTreeIsInitialized() {
      var merkleTreeAccountInfo = await this.merkleTreeProgram.account.merkleTree.fetch(
        this.merkleTreePubkey
      )
      assert(merkleTreeAccountInfo != null, "merkleTreeAccountInfo not initialized")
      // zero values
      // index == 0
      // roots are empty save for 0
      // lock duration is correct
      assert(merkleTreeAccountInfo.lockDuration.toString() == "50")

    }

    async checkPreInsertedLeavesIndexIsInitialized() {
      var preInsertedLeavesIndexAccountInfo = await this.merkleTreeProgram.account.preInsertedLeavesIndex.fetch(
        this.preInsertedLeavesIndex
      )

      assert(preInsertedLeavesIndexAccountInfo != null, "preInsertedLeavesIndexAccountInfo not initialized")
      assert(preInsertedLeavesIndexAccountInfo.nextIndex.toString() == "0");
    }

    async printMerkleTree() {
      var merkleTreeAccountInfo = await this.merkleTreeProgram.account.merkleTree.fetch(
        MERKLE_TREE_KEY
      )
      console.log("merkleTreeAccountInfo ", merkleTreeAccountInfo);

    }

    async getMerkleTreeAuthorityPda() {
      this.merkleTreeAuthorityPda = (await solana.PublicKey.findProgramAddress(
          [anchor.utils.bytes.utf8.encode("MERKLE_TREE_AUTHORITY")],
          this.merkleTreeProgram.programId
        ))[0];
      return this.merkleTreeAuthorityPda;
    }

    async initMerkleTreeAuthority(authority) {
      if (authority == undefined) {
        authority = this.payer
      }
      if (this.merkleTreeAuthorityPda == undefined) {
        await this.getMerkleTreeAuthorityPda()
      }

      const tx = await this.merkleTreeProgram.methods.initializeMerkleTreeAuthority().accounts({
        authority: authority.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        ...DEFAULT_PROGRAMS
      })
      .signers([authority])
      .rpc({commitment: "finalized", preflightCommitment: 'finalized',});

      let merkleTreeAuthority = await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(this.merkleTreeAuthorityPda)
      assert(merkleTreeAuthority.enablePermissionlessSplTokens == false);
      assert(merkleTreeAuthority.enableNfts == false);
      assert(merkleTreeAuthority.pubkey.toBase58() ==  authority.publicKey.toBase58());
      assert(merkleTreeAuthority.registeredAssetIndex.toString() ==  "0");

      return tx;
    }

    async updateMerkleTreeAuthority(newAuthority, test = false) {
      if (this.merkleTreeAuthorityPda == undefined) {
        await this.getMerkleTreeAuthorityPda()
      }
      let merkleTreeAuthorityPrior
      if (test != true) {
        merkleTreeAuthorityPrior = await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(this.merkleTreeAuthorityPda)
        if (merkleTreeAuthorityPrior == null) {
          throw `Merkle tree authority ${this.merkleTreeAuthorityPda.toBase58()} not initialized`
        }

      }

      const tx = await this.merkleTreeProgram.methods.updateMerkleTreeAuthority().accounts({
        authority: this.payer.publicKey,
        newAuthority,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        ...DEFAULT_PROGRAMS
      })
      .signers([this.payer])
      .rpc({commitment: "finalized", preflightCommitment: 'finalized',});

      if (test != true) {
        let merkleTreeAuthority = await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(this.merkleTreeAuthorityPda)

        assert(merkleTreeAuthority.enablePermissionlessSplTokens == merkleTreeAuthorityPrior.enablePermissionlessSplTokens);
        assert(merkleTreeAuthority.enableNfts == merkleTreeAuthorityPrior.enableNfts);
        assert(merkleTreeAuthority.pubkey.toBase58() ==  newAuthority.toBase58());
      }
      return tx;
    }

    async enableNfts(configValue) {
      if (this.merkleTreeAuthorityPda == undefined) {
        await this.getMerkleTreeAuthorityPda()
      }
      const tx = await this.merkleTreeProgram.methods.enableNfts(configValue).accounts({
        authority: this.payer.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        ...DEFAULT_PROGRAMS
      })
      .signers([this.payer])
      .rpc({commitment: "finalized", preflightCommitment: 'finalized',});
      let merkleTreeAuthority = await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(this.merkleTreeAuthorityPda)
      assert(merkleTreeAuthority.enableNfts == configValue);

      return tx;
    }

    async enablePermissionlessSplTokens(configValue) {
      if (this.merkleTreeAuthorityPda == undefined) {
        await this.getMerkleTreeAuthorityPda()
      }
      const tx = await this.merkleTreeProgram.methods.enablePermissionlessSplTokens(configValue).accounts({
        authority: this.payer.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        ...DEFAULT_PROGRAMS
      })
      .signers([this.payer])
      .rpc({commitment: "finalized", preflightCommitment: 'finalized',});
      let merkleTreeAuthority = await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(this.merkleTreeAuthorityPda)
      assert(merkleTreeAuthority.enablePermissionlessSplTokens == configValue);
      return tx;
    }

    async updateLockDuration(lockDuration: Number) {
      if (this.merkleTreeAuthorityPda == undefined) {
        await this.getMerkleTreeAuthorityPda()
      }

      const tx = await this.merkleTreeProgram.methods.updateLockDuration(new anchor.BN(lockDuration.toString())).accounts({
        authority: this.payer.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        merkleTree: this.merkleTreePubkey,
        ...DEFAULT_PROGRAMS
      })
      .signers([this.payer])
      .rpc({commitment: "finalized", preflightCommitment: 'finalized',});
      let merkleTree = await this.merkleTreeProgram.account.merkleTree.fetch(this.merkleTreePubkey)
      assert(merkleTree.lockDuration == lockDuration);
      console.log("lock duration updated to: ", lockDuration);

      return tx;
    }

    async getRegisteredVerifierPda(verifierPubkey) {
      // TODO: add check whether already exists
      this.registeredVerifierPdas.push({registeredVerifierPda: (await solana.PublicKey.findProgramAddress(
          [verifierPubkey.toBuffer()],
          this.merkleTreeProgram.programId
        ))[0],
        verifierPubkey: verifierPubkey
      });
      return this.registeredVerifierPdas[this.registeredVerifierPdas.length - 1];
    }

    async registerVerifier(verifierPubkey) {
      let registeredVerifierPda = this.registeredVerifierPdas.filter((item) => {return item.verifierPubkey === verifierPubkey})[0]

      if (!registeredVerifierPda) {
          registeredVerifierPda = await this.getRegisteredVerifierPda(verifierPubkey)
      }

      const tx = await this.merkleTreeProgram.methods.registerVerifier(
        verifierPubkey
      ).accounts({
        registeredVerifierPda: registeredVerifierPda.registeredVerifierPda,
        authority: this.payer.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        ...DEFAULT_PROGRAMS
      })
      .signers([this.payer])
      .rpc({commitment: "finalized", preflightCommitment: 'finalized',});

      await this.checkVerifierIsRegistered(verifierPubkey);

      return tx;
    }

    async checkVerifierIsRegistered(verifierPubkey) {

      let registeredVerifierPda = this.registeredVerifierPdas.filter((item) => {return item.verifierPubkey === verifierPubkey})[0]

      var registeredVerifierAccountInfo = await this.merkleTreeProgram.account.registeredVerifier.fetch(
        registeredVerifierPda.registeredVerifierPda
      )

      assert(registeredVerifierAccountInfo != null);
      assert(registeredVerifierAccountInfo.pubkey.toBase58() == verifierPubkey.toBase58());

    }

    async getPoolTypePda(poolType) {
      if (poolType.length != 32) {
        throw `invalid pooltype length ${poolType.length}`
      }
      // TODO: add check whether already exists

      this.poolTypes.push({poolPda: (await solana.PublicKey.findProgramAddress(
          [poolType, anchor.utils.bytes.utf8.encode("pooltype")],
          this.merkleTreeProgram.programId
        ))[0], poolType: poolType});
        return this.poolTypes[this.poolTypes.length - 1];
    }

    async registerPoolType(poolType) {
      let registeredPoolTypePda = this.poolTypes.filter((item) => {return item.poolType === poolType})[0]

      if (!registeredPoolTypePda) {
          registeredPoolTypePda = await this.getPoolTypePda(poolType)
      }

      const tx = await this.merkleTreeProgram.methods.registerPoolType(
        Buffer.from(new Uint8Array(32).fill(0))
      ).accounts({
        registeredPoolTypePda:  registeredPoolTypePda.poolPda,
        authority:              this.payer.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        ...DEFAULT_PROGRAMS
      })
      .signers([this.payer])
      .rpc({commitment: "confirmed", preflightCommitment: 'confirmed',});
      return tx;
    }

    async checkPoolRegistered(poolPda, poolType, mint = null) {
      var registeredTokenConfigAccount = await this.merkleTreeProgram.account.registeredAssetPool.fetch(
        poolPda.pda
      )

      var merkleTreeAuthorityPdaAccountInfo = await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(
        this.merkleTreeAuthorityPda
      )
      assert(registeredTokenConfigAccount.poolType.toString() == poolType.toString())
      assert(registeredTokenConfigAccount.index.toString() == (merkleTreeAuthorityPdaAccountInfo.registeredAssetIndex - 1).toString())


      if (mint !== null) {

        assert(registeredTokenConfigAccount.assetPoolPubkey.toBase58() == poolPda.token.toBase58())

        var registeredTokenAccount = await token.getAccount(this.connection,
          poolPda.token, {commitment: "confirmed", preflightCommitment: 'confirmed'}
        )
        assert(registeredTokenAccount != null);
        assert(registeredTokenAccount.mint.toBase58() == mint.toBase58());

      } else {
        assert(registeredTokenConfigAccount.assetPoolPubkey.toBase58() == poolPda.pda.toBase58())

      }

    }

    async getSolPoolPda(poolType) {
      this.poolPdas.push({pda: (await solana.PublicKey.findProgramAddress(
        [new Uint8Array(32).fill(0), poolType, anchor.utils.bytes.utf8.encode("pool-config")],
          this.merkleTreeProgram.programId
        ))[0], poolType: poolType});
        return this.poolPdas[this.poolPdas.length - 1]
    }

    async registerSolPool(poolType) {
      let registeredPoolTypePda = this.poolTypes.filter((item) => {return item.poolType === poolType})[0]

      if (!registeredPoolTypePda) {
          registeredPoolTypePda = await this.getPoolTypePda(poolType)
      }
      let solPoolPda = await this.getSolPoolPda(poolType);


      const tx = await this.merkleTreeProgram.methods.registerSolPool(
      ).accounts({
        registeredAssetPoolPda:  solPoolPda.pda,
        authority:              this.payer.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        registeredPoolTypePda:  registeredPoolTypePda.poolPda,
        ...DEFAULT_PROGRAMS
      })
      .signers([this.payer])
      .rpc({commitment: "confirmed", preflightCommitment: 'confirmed'});

      await this.checkPoolRegistered(solPoolPda, poolType)

      return tx;
    }

    async getSplPoolPdaToken(poolType, mint) {
      let pda = (await solana.PublicKey.findProgramAddress(
        [mint.toBytes(), new Uint8Array(32).fill(0), anchor.utils.bytes.utf8.encode("pool")],
          this.merkleTreeProgram.programId
        ))[0];
        return pda;
    }

    async getSplPoolPda(poolType, mint) {
      this.poolPdas.push({pda: (await solana.PublicKey.findProgramAddress(
        [mint.toBytes(), new Uint8Array(32).fill(0), anchor.utils.bytes.utf8.encode("pool-config")],
          this.merkleTreeProgram.programId
        ))[0], poolType: poolType, token: await this.getSplPoolPdaToken(poolType, mint)});
      return this.poolPdas[this.poolPdas.length - 1];
    }

    async getTokenAuthority() {
      this.tokenAuthority = (await solana.PublicKey.findProgramAddress(
        [anchor.utils.bytes.utf8.encode("spl")],
        this.merkleTreeProgram.programId
      ))[0];
      return this.tokenAuthority;
    }

    async registerSplPool(poolType, mint) {
      let registeredPoolTypePda = this.poolTypes.filter((item) => {return item.poolType === poolType})[0]

      if (!registeredPoolTypePda) {
          registeredPoolTypePda = await this.getPoolTypePda(poolType)
      }


      let splPoolPda = await this.getSplPoolPda(poolType, mint);

      if (!this.tokenAuthority) {
          await this.getTokenAuthority()
      }


      const tx = await this.merkleTreeProgram.methods.registerSplPool(
      ).accounts({
        registeredAssetPoolPda:  splPoolPda.pda,
        authority:              this.payer.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        registeredPoolTypePda:  registeredPoolTypePda.poolPda,
        merkleTreePdaToken: splPoolPda.token,
        tokenAuthority: this.tokenAuthority,
        mint,
        ...DEFAULT_PROGRAMS
      })
      .signers([this.payer])
      .rpc({commitment: "confirmed", preflightCommitment: 'confirmed',});

      await this.checkPoolRegistered(splPoolPda, poolType, mint)

      return tx;
    }
}

export async function getUninsertedLeaves({
  merkleTreeProgram,
  merkleTreeIndex,
  connection
  // merkleTreePubkey
}) {
  var leave_accounts: Array<{
    pubkey: PublicKey
    account: Account<Buffer>
  }> = await merkleTreeProgram.account.twoLeavesBytesPda.all();
  console.log("Total nr of accounts. ", leave_accounts.length);

  let filteredLeaves = leave_accounts
  .filter((pda) => {
    return pda.account.leftLeafIndex.toNumber() >= merkleTreeIndex.toNumber()
  }).sort((a, b) => a.account.leftLeafIndex.toNumber() - b.account.leftLeafIndex.toNumber());

  return filteredLeaves.map((pda) => {
      return { isSigner: false, isWritable: false, pubkey: pda.publicKey};
  })
}

export async function getUnspentUtxo(leavesPdas, provider, encryptionKeypair, KEYPAIR, FEE_ASSET,MINT_CIRCUIT, POSEIDON, merkleTreeProgram) {
  let decryptedUtxo1
  for (var i = 0; i < leavesPdas.length; i++) {
    console.log("iter ", i);

    // decrypt first leaves account and build utxo
    decryptedUtxo1 = light.Utxo.decrypt(new Uint8Array(Array.from(leavesPdas[i].account.encryptedUtxos.slice(0,63))), new Uint8Array(Array.from(leavesPdas[i].account.encryptedUtxos.slice(63, 87))), encryptionKeypair.publicKey, encryptionKeypair, KEYPAIR, [FEE_ASSET,MINT_CIRCUIT], POSEIDON)[1];
    let nullifier = decryptedUtxo1.getNullifier();

    let nullifierPubkey = (await solana.PublicKey.findProgramAddress(
        [new anchor.BN(nullifier.toString()).toBuffer(), anchor.utils.bytes.utf8.encode("nf")],
        merkleTreeProgram.programId))[0]
    let accountInfo = await provider.connection.getAccountInfo(nullifierPubkey);

    if (accountInfo == null && decryptedUtxo1.amounts[1].toString() != "0" && decryptedUtxo1.amounts[0].toString() != "0") {
      console.log("found unspent leaf");
      return decryptedUtxo1;
    } else if (i == leavesPdas.length - 1) {
      throw "no unspent leaf found";
    }

  }

}

export async function getInsertedLeaves({
  merkleTreeProgram,
  merkleTreeIndex,
  connection
  // merkleTreePubkey
}) {
  var leave_accounts: Array<{
    pubkey: PublicKey
    account: Account<Buffer>
  }> = await merkleTreeProgram.account.twoLeavesBytesPda.all();
  console.log("Total nr of accounts. ", leave_accounts.length);

  let filteredLeaves = leave_accounts
  .filter((pda) => {
    return pda.account.leftLeafIndex.toNumber() < merkleTreeIndex.toNumber()
  }).sort((a, b) => a.account.leftLeafIndex.toNumber() - b.account.leftLeafIndex.toNumber());

  return filteredLeaves;
}
