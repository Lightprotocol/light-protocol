const light = require('../../light-protocol-sdk');
const { U64, I64 } = require('n64');
import * as anchor from '@project-serum/anchor';

const FIELD_SIZE = new anchor.BN(
  '21888242871839275222246405745257275088548364400416034343698204186575808495617'
);

var assert = require('assert');
let circomlibjs = require('circomlibjs');

import { DEFAULT_PROGRAMS } from './constants';
import {
  PublicKey,
  SystemProgram,
  TransactionMessage,
  AddressLookupTableAccount,
  VersionedTransaction,
  sendAndConfirmRawTransaction,
  Keypair,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, getAccount } from '@solana/spl-token';
import { checkRentExemption } from './test_checks';
import { unpackLeavesAccount } from './unpack_accounts';
import nacl from 'tweetnacl';
import { Utxo } from '../../light-protocol-sdk/lib';
import MerkleTree from '../../light-protocol-sdk/lib/merkelTree';

export class shieldedTransaction {
  relayerPubkey: PublicKey;
  relayerRecipient: PublicKey;
  preInsertedLeavesIndex: any;
  merkleTreeProgram: anchor.Program;
  verifierProgram: anchor.Program;
  provider: anchor.Provider;
  lookupTable: PublicKey;
  feeAsset: anchor.BN;
  relayerFee: any; // U64 
  merkleTreeIndex: number;
  merkleTreePubkey: PublicKey;
  utxos: Utxo[];
  feeUtxos: Utxo[];
  encryptionKeypair: nacl.BoxKeyPair;
  payer: Keypair;
  recipient: PublicKey;
  shieldedKeypair: any; // light.Keypair
  merkleTree: MerkleTree;
  merkleTreeFeeAssetPubkey: PublicKey;
  merkleTreeAssetPubkey: PublicKey;
  constructor({
    user,
    relayerFee = U64(10_000),
    merkleTreeIndex = 0,
    merkleTree = null,
    merkleTreeAssetPubkey = null,
    recipient = null, //PublicKey
    // recipientFee: number,
    lookupTable, //PublicKey
    payer, //: Keypair
    relayerPubkey = null, //PublicKey
    config,
    merkleTreePubkey, // can be added to config
    preInsertedLeavesIndex,
    merkleTreeFeeAssetPubkey,
    relayerRecipient,
  }) {
    if (relayerPubkey == null) {
      this.relayerPubkey = new PublicKey(payer.publicKey);
    } else {
      this.relayerPubkey = relayerPubkey;
    }
    this.relayerRecipient = relayerRecipient;
    this.preInsertedLeavesIndex = preInsertedLeavesIndex;
    this.merkleTreeProgram = config.merkleTreeProgram;
    console.log('merkleTreeProgram', this.merkleTreeProgram);
    this.verifierProgram = config.verifierProgram;
    console.log('verifierProgram', this.verifierProgram);
    this.provider = config.provider;
    console.log('provider', this.provider);
    this.lookupTable = lookupTable;
    this.feeAsset = new anchor.BN(
      anchor.web3.SystemProgram.programId._bn.toString()
    ).mod(FIELD_SIZE);
    this.relayerFee = relayerFee;
    this.merkleTreeIndex = merkleTreeIndex;
    this.merkleTreePubkey = merkleTreePubkey;
    this.merkleTreeAssetPubkey = merkleTreeAssetPubkey;
    this.merkleTree = null;
    this.utxos = [];
    this.feeUtxos = [];
    this.encryptionKeypair = user.encryptionKeypair;
    console.log('encryptionKeypair', this.encryptionKeypair);
    this.shieldedKeypair = user.shieldedKeypair;
    console.log('shieldedKeypair', this.shieldedKeypair);
    this.payer = payer;
    console.log('payer: ', payer);

    this.recipient = recipient;
    this.merkleTreeFeeAssetPubkey = merkleTreeFeeAssetPubkey;
  }


  async shield({
    userTokenAccount,
    AUTHORITY,
    outputUtxos,
    FEE_ASSET,
    ASSET,
    ASSET_1,
    depositFeeAmount,
  }: {
    userTokenAccount: PublicKey;
    AUTHORITY: PublicKey;
    outputUtxos: Utxo[];
    FEE_ASSET: anchor.BN;
    ASSET: anchor.BN;
    ASSET_1: anchor.BN;
    depositFeeAmount: number;
  }) {
    console.log('getMerkleTree');
    await this.getMerkleTree();
    console.log('prepareTranscationFull');
    await this.prepareTransactionFull({
      inputUtxos: [],
      outputUtxos,
      action: 'DEPOSIT',
      assetPubkeys: [FEE_ASSET, ASSET, ASSET_1],
      relayerFee: U64(depositFeeAmount),
      shuffle: true,
      mintPubkey: ASSET,
      sender: userTokenAccount,
    });
    console.log('proof');
    await this.proof();
    console.log('sendTransaction');
    try {
      let res = await this.sendTransaction();
      console.log(res);
    } catch (e) {
      console.log(e);
      console.log('AUTHORITY: ', AUTHORITY.toBase58());
    }
    try {
      await this.checkBalances();
    } catch (e) {
      console.log(e);
    }
  }

  async getMerkleTree() {
    this.poseidon = await circomlibjs.buildPoseidonOpt();
    if (this.shieldedKeypair == null) {
      this.shieldedKeypair = new light.Keypair(this.poseidon);
    }
    this.merkleTree = await light.buildMerkelTree(this.poseidon);
    this.merkleTreeLeavesIndex = 0;
  }

  async prepareTransaction() {
    let data = await light.prepareTransaction(
      this.inputUtxos,
      this.outputUtxos,
      this.merkleTree,
      this.merkleTreeIndex,
      this.merkleTreePubkey.toBytes(),
      this.externalAmountBigNumber,
      this.relayerFee,
      this.recipient, // recipient
      this.relayerPubkey,
      this.action,
      this.encryptionKeypair,
      this.inIndices,
      this.outIndices,
      this.assetPubkeys,
      this.mintPubkey,
      false,
      this.feeAmount,
      this.recipientFee
    );
    this.input = data.input;
    this.extAmount = data.extAmount;
    this.externalAmountBigNumber = data.externalAmountBigNumber;
    this.extDataBytes = data.extDataBytes;
    this.encryptedOutputs = data.extDataBytes;
  }

  async prepareTransactionFull({
    inputUtxos,
    outputUtxos,
    action,
    assetPubkeys,
    recipient = null,
    mintPubkey = new anchor.BN(0),
    relayerFee = null, // public amount of the fee utxo adjustable if you want to deposit a fee utxo alongside your spl deposit
    shuffle = true,
    recipientFee = null,
    sender,
  }: {
    inputUtxos: Utxo[];
    outputUtxos: Utxo[];
    action: string;
    assetPubkeys: anchor.BN[];
    recipient: PublicKey;
    mintPubkey: anchor.BN;
    relayerFee: any //U64;
    shuffle: boolean;
    recipientFee: any; //U64(?);
    sender: PublicKey;
  }) {
    mintPubkey = assetPubkeys[1];
    if (assetPubkeys[0].toString() != this.feeAsset.toString()) {
      throw 'feeAsset should be assetPubkeys[0]';
    }
    if (action == 'DEPOSIT') {
      this.relayerFee = relayerFee;
      console.log(this.payer);

      this.sender = sender;
      console.log('setting recipient to this.merkleTreeAssetPubkey');
      this.senderFee = new PublicKey(this.payer.publicKey);
      this.recipient = this.merkleTreeAssetPubkey;
      console.log('this.recipient: ', this.recipient);
      console.log('this.merkleTreeAssetPubkey: ', this.merkleTreeAssetPubkey);
      this.recipientFee = this.merkleTreeFeeAssetPubkey;

      if (
        this.relayerPubkey.toBase58() !=
        new PublicKey(this.payer.publicKey).toBase58()
      ) {
        throw 'relayerPubkey and payer pubkey need to be equivalent at deposit';
      }
    } else if (action == 'WITHDRAWAL') {
      this.senderFee = this.merkleTreeFeeAssetPubkey;
      this.recipientFee = recipientFee;
      this.sender = this.merkleTreeAssetPubkey;
      this.recipient = recipient;
      if (relayerFee != null) {
        this.relayerFee = relayerFee;
        if (relayerFee == undefined) {
          throw 'relayerFee undefined';
        }
      }

      if (recipient == undefined) {
        throw 'recipient undefined';
      }
      if (recipientFee == undefined) {
        throw 'recipientFee undefined';
      }
    }

    console.log('this.recipientFee: ', this.recipientFee);

    this.assetPubkeys = assetPubkeys;
    this.mintPubkey = mintPubkey;
    this.action = action;

    let res = light.prepareUtxos(
      inputUtxos,
      outputUtxos,
      this.relayerFee,
      this.assetPubkeys,
      this.action,
      this.poseidon,
      shuffle
    );
    console.log(' light.prepareUtxos res ', res);

    this.inputUtxos = res.inputUtxos;
    this.outputUtxos = res.outputUtxos;
    this.inIndices = res.inIndices;
    this.outIndices = res.outIndices;
    this.externalAmountBigNumber = res.externalAmountBigNumber;
    this.feeAmount = res.feeAmount;

    let data = await light.prepareTransaction(
      this.inputUtxos,
      this.outputUtxos,
      this.merkleTree,
      this.merkleTreeIndex,
      this.merkleTreePubkey.toBytes(),
      this.externalAmountBigNumber,
      this.relayerFee,
      this.recipient,
      this.relayerPubkey,
      this.action,
      this.encryptionKeypair,
      this.inIndices,
      this.outIndices,
      this.assetPubkeys,
      this.mintPubkey,
      false,
      this.feeAmount,
      this.recipientFee
    );
    this.input = data.input;
    assert(this.input.mintPubkey == this.mintPubkey);
    assert(this.input.mintPubkey == this.assetPubkeys[1]);
    this.extAmount = data.extAmount;
    this.externalAmountBigNumber = data.externalAmountBigNumber;
    this.extDataBytes = data.extDataBytes;
    this.encrypedUtxos = data.encryptedUtxos;
    if (this.externalAmountBigNumber != 0) {
      if (assetPubkeys[1].toString() != mintPubkey.toString()) {
        throw 'mintPubkey should be assetPubkeys[1]';
      }
    }
  }

  async proof() {
    if (this.merkleTree == null) {
      throw 'merkle tree not built';
    }
    if (this.inIndices == null) {
      throw 'transaction not prepared';
    }
    let proofData = await light.getProofMasp(
      this.input,
      this.extAmount,
      this.externalAmountBigNumber,
      this.extDataBytes,
      this.encrypedUtxos
    );

    this.proofData = proofData;

    let pdas = await getPdaAddresses({
      tx_integrity_hash: this.proofData.publicInputs.txIntegrityHash,
      nullifiers: [
        this.proofData.publicInputs.nullifier0,
        this.proofData.publicInputs.nullifier1,
      ],
      leftLeaves: [this.proofData.publicInputs.leafLeft],
      merkleTreeProgram: this.merkleTreeProgram,
      verifierProgram: this.verifierProgram,
    });
    this.escrow = pdas.escrow;
    this.leavesPdaPubkeys = pdas.leavesPdaPubkeys;
    this.nullifierPdaPubkeys = pdas.nullifierPdaPubkeys;
    this.signerAuthorityPubkey = pdas.signerAuthorityPubkey;
    this.tokenAuthority = pdas.tokenAuthority;
    return this.proofData;
  }

  async sendTransaction(insert = true) {
    console.log('this.relayerFee ', this.relayerFee);

    this.recipientBalancePriorTx = (
      await getAccount(
        this.provider.connection,
        this.recipient,
        TOKEN_PROGRAM_ID
      )
    ).amount;
    this.recipientFeeBalancePriorTx = await this.provider.connection.getBalance(
      this.recipientFee
    );
    console.log('recipientBalancePriorTx: ', this.recipientBalancePriorTx);
    console.log(
      'recipientFeeBalancePriorTx: ',
      this.recipientFeeBalancePriorTx
    );
    console.log('sender_fee: ', this.senderFee);
    this.senderFeeBalancePriorTx = await this.provider.connection.getBalance(
      this.senderFee
    );
    this.relayerRecipientAccountBalancePriorLastTx =
      await this.provider.connection.getBalance(this.relayerRecipient);
    console.log(
      'relayerAccountBalancePriorLastTx: ',
      this.relayerRecipientAccountBalancePriorLastTx
    );

    const ix = await this.verifierProgram.methods
      .shieldedTransferInputs(
        this.proofData.proofBytes,
        this.proofData.publicInputs.root,
        this.proofData.publicInputs.publicAmount,
        this.proofData.publicInputs.extDataHash,
        [
          this.proofData.publicInputs.nullifier0,
          this.proofData.publicInputs.nullifier1,
        ],
        [
          this.proofData.publicInputs.leafRight,
          this.proofData.publicInputs.leafLeft,
        ],
        this.proofData.publicInputs.feeAmount,
        this.proofData.publicInputs.mintPubkey,
        new anchor.BN(1),
        new anchor.BN(0),
        new anchor.BN(this.relayerFee.toString()),
        this.proofData.encryptedOutputs.slice(0, 128),
        this.proofData.encryptedOutputs.slice(128, 192),
        this.proofData.encryptedOutputs.slice(192, 224),
        this.proofData.encryptedOutputs.slice(224, 238)
      )
      .accounts({
        signingAddress: this.relayerPubkey,
        systemProgram: SystemProgram.programId,
        programMerkleTree: this.merkleTreeProgram.programId,
        rent: DEFAULT_PROGRAMS.rent,
        merkleTree: this.merkleTreePubkey,
        preInsertedLeavesIndex: this.preInsertedLeavesIndex,
        authority: this.signerAuthorityPubkey,
        tokenProgram: TOKEN_PROGRAM_ID,
        sender: this.sender,
        recipient: this.recipient,
        senderFee: this.senderFee,
        recipientFee: this.recipientFee,
        relayerRecipient: this.relayerRecipient,
        escrow: this.escrow,
        tokenAuthority: this.tokenAuthority,
      })
      .remainingAccounts([
        {
          isSigner: false,
          isWritable: true,
          pubkey: this.nullifierPdaPubkeys[0],
        },
        {
          isSigner: false,
          isWritable: true,
          pubkey: this.nullifierPdaPubkeys[1],
        },
        { isSigner: false, isWritable: true, pubkey: this.leavesPdaPubkeys[0] },
      ])
      .signers([this.payer])
      .instruction();
    console.log('this.payer: ', this.payer);

    let recentBlockhash = (await this.provider.connection.getRecentBlockhash())
      .blockhash;
    let txMsg = new TransactionMessage({
      payerKey: this.payer.publicKey,
      instructions: [
        // ComputeBudgetProgram.requestUnits({units:1_400_000, fee: 1}),
        // SystemProgram.transfer({fromPubkey:payer.publicKey, toPubkey: AUTHORITY, lamports: 3173760 * 3}),
        ix,
      ],
      recentBlockhash: recentBlockhash,
    });
    console.log(this.lookupTable.toBase58());

    let lookupTableAccount = await this.provider.connection.getAccountInfo(
      this.lookupTable,
      'confirmed'
    );
    console.log('lookupTableAccount: ', lookupTableAccount);

    let unpackedLookupTableAccount = AddressLookupTableAccount.deserialize(
      lookupTableAccount.data
    );
    console.log('unpackedLookupTableAccount ', unpackedLookupTableAccount);

    let compiledTx = txMsg.compileToV0Message([
      { state: unpackedLookupTableAccount },
    ]);
    compiledTx.addressTableLookups[0].accountKey = this.lookupTable;

    let transaction = new VersionedTransaction(compiledTx);
    let retries = 3;
    while (retries > 0) {
      transaction.sign([this.payer]);
      console.log(transaction);
      console.log(transaction.message.addressTableLookups);
      recentBlockhash = (await this.provider.connection.getRecentBlockhash())
        .blockhash;
      transaction.message.recentBlockhash = recentBlockhash;
      let serializedTx = transaction.serialize();
      // console.log(this.provider.connection);
      let res;

      try {
        console.log('serializedTx: ');

        res = await sendAndConfirmRawTransaction(
          this.provider.connection,
          serializedTx,
          {
            commitment: 'finalized',
            preflightCommitment: 'finalized',
          }
        );
        retries = 0;
      } catch (e) {
        console.log(e);
        retries--;
      }
    }

    // storing utxos
    this.outputUtxos.map((utxo) => {
      if (utxo.amounts[1] != 0 && utxo.assets[1] != this.feeAsset) {
        this.utxos.push(utxo);
      }
      if (
        utxo.amounts[0] != 0 &&
        utxo.assets[0].toString() == this.feeAsset.toString()
      ) {
        this.feeUtxos.push(utxo);
      }
    });
    this.inIndices = null;
    // inserting output utxos into merkle tree
    if (insert != 'NOINSERT') {
      for (var i = 0; i < this.outputUtxos.length; i++) {
        this.merkleTree.update(
          this.merkleTreeLeavesIndex,
          this.outputUtxos[i].getCommitment()
        );
        this.merkleTreeLeavesIndex++;
      }
    }

    return res;
  }

  async checkBalances() {
    // Checking that nullifiers were inserted
    this.is_token = true;

    for (var i in this.nullifierPdaPubkeys) {
      console.log(i);

      var nullifierAccount = await this.provider.connection.getAccountInfo(
        this.nullifierPdaPubkeys[i],
        {
          commitment: 'confirmed',
          preflightCommitment: 'confirmed',
        }
      );
      console.log(nullifierAccount);

      await checkRentExemption({
        account: nullifierAccount,
        connection: this.provider.connection,
      });
    }
    let leavesAccount;
    var leavesAccountData;
    // Checking that leaves were inserted
    for (var i in this.leavesPdaPubkeys) {
      console.log(i);

      leavesAccount = await this.provider.connection.getAccountInfo(
        this.leavesPdaPubkeys[i]
      );

      leavesAccountData = unpackLeavesAccount(leavesAccount.data);
      await checkRentExemption({
        account: leavesAccount,
        connection: this.provider.connection,
      });
      try {
        console.log(leavesAccountData);

        assert(
          leavesAccountData.leafLeft.toString() ===
            this.proofData.publicInputs.leafLeft.toString(),
          'left leaf not inserted correctly'
        );
        assert(
          leavesAccountData.leafRight.toString() ===
            this.proofData.publicInputs.leafRight.toString(),
          'right leaf not inserted correctly'
        );
        assert(
          leavesAccountData.encryptedUtxos.toString() ===
            this.encrypedUtxos.toString(),
          'encryptedUtxos not inserted correctly'
        );
        assert(leavesAccountData.leafType === 7);
      } catch (e) {
        console.log('leaves: ', e);
      }
    }

    console.log('here2');
    console.log(`mode ${this.action}, this.is_token ${this.is_token}`);

    try {
      console.log('this.preInsertedLeavesIndex ', this.preInsertedLeavesIndex);

      var preInsertedLeavesIndexAccount =
        await this.provider.connection.getAccountInfo(
          this.preInsertedLeavesIndex
        );

      console.log(preInsertedLeavesIndexAccount);
      const preInsertedLeavesIndexAccountAfterUpdate =
        this.merkleTreeProgram.account.preInsertedLeavesIndex._coder.accounts.decode(
          'PreInsertedLeavesIndex',
          preInsertedLeavesIndexAccount.data
        );

      assert(
        Number(preInsertedLeavesIndexAccountAfterUpdate.nextIndex) ==
          Number(leavesAccountData.leafIndex) + this.leavesPdaPubkeys.length * 2
      );
    } catch (e) {
      console.log('preInsertedLeavesIndex: ', e);
    }

    if (this.action == 'WITHDRAWAL') {
    }

    if (this.action == 'DEPOSIT' && this.is_token == false) {
      var recipientAccount = await this.provider.connection.getAccountInfo(
        this.recipient
      );
      assert(
        recipientAccount.lamports ==
          I64(this.recipientBalancePriorTx)
            .add(this.proofData.externalAmountBigNumber.toString())
            .toString(),
        'amount not transferred correctly'
      );
    } else if (this.action == 'DEPOSIT' && this.is_token == true) {
      console.log('DEPOSIT and token');
      console.log('this.recipient: ', this.recipient);

      var recipientAccount = await getAccount(
        this.provider.connection,
        this.recipient,
        TOKEN_PROGRAM_ID
      );
      var recipientFeeAccountBalance =
        await this.provider.connection.getBalance(this.recipientFee);

      console.log(
        `Balance now ${recipientAccount.amount} balance beginning ${this.recipientBalancePriorTx}`
      );
      console.log(
        `Balance now ${recipientAccount.amount} balance beginning ${
          Number(this.recipientBalancePriorTx) +
          Number(this.proofData.externalAmountBigNumber)
        }`
      );
      assert(
        recipientAccount.amount ==
          (
            Number(this.recipientBalancePriorTx) +
            Number(this.proofData.externalAmountBigNumber)
          ).toString(),
        'amount not transferred correctly'
      );
      console.log(
        `Blanace now ${recipientFeeAccountBalance} ${
          Number(this.recipientFeeBalancePriorTx) + Number(this.feeAmount)
        }`
      );
      console.log('fee amount: ', this.feeAmount);
      console.log(
        'fee amount from inputs. ',
        new anchor.BN(
          this.proofData.publicInputs.feeAmount.slice(24, 32)
        ).toString()
      );
      console.log(
        'pub amount from inputs. ',
        new anchor.BN(
          this.proofData.publicInputs.publicAmount.slice(24, 32)
        ).toString()
      );

      console.log(
        'recipientFeeBalancePriorTx: ',
        this.recipientFeeBalancePriorTx
      );

      var senderFeeAccountBalance = await this.provider.connection.getBalance(
        this.senderFee
      );
      console.log('senderFeeAccountBalance: ', senderFeeAccountBalance);
      console.log(
        'this.senderFeeBalancePriorTx: ',
        this.senderFeeBalancePriorTx
      );

      assert(
        recipientFeeAccountBalance ==
          Number(this.recipientFeeBalancePriorTx) + Number(this.feeAmount)
      );
      console.log(
        `${Number(this.senderFeeBalancePriorTx)} - ${Number(
          this.feeAmount
        )} == ${senderFeeAccountBalance}`
      );
      assert(
        Number(this.senderFeeBalancePriorTx) - Number(this.feeAmount) - 5000 ==
          Number(senderFeeAccountBalance)
      );
    } else if (this.action == 'WITHDRAWAL' && this.is_token == false) {
      var senderAccount = await this.provider.connection.getAccountInfo(
        this.sender
      );
      var recipientAccount = await this.provider.connection.getAccountInfo(
        this.recipient
      );
      assert(
        senderAccount.lamports ==
          I64(senderAccountBalancePriorLastTx)
            .add(I64.readLE(this.proofData.extAmount, 0))
            .sub(I64(relayerFee))
            .toString(),
        'amount not transferred correctly'
      );

      var recipientAccount = await this.provider.connection.getAccountInfo(
        recipient
      );

      assert(
        recipientAccount.lamports ==
          I64(Number(this.recipientBalancePriorTx))
            .sub(I64.readLE(this.proofData.extAmount, 0))
            .toString(),
        'amount not transferred correctly'
      );
    } else if (this.action == 'WITHDRAWAL' && this.is_token == true) {
      var senderAccount = await getAccount(
        this.provider.connection,
        this.sender,
        TOKEN_PROGRAM_ID
      );
      var recipientAccount = await getAccount(
        this.provider.connection,
        this.recipient,
        TOKEN_PROGRAM_ID
      );

      console.log(
        `${recipientAccount.amount}, ${new anchor.BN(
          this.recipientBalancePriorTx
        )
          .sub(this.proofData.externalAmountBigNumber)
          .toString()}`
      );
      assert(
        recipientAccount.amount.toString() ==
          new anchor.BN(this.recipientBalancePriorTx)
            .sub(this.proofData.externalAmountBigNumber)
            .toString(),
        'amount not transferred correctly'
      );

      var relayerAccount = await this.provider.connection.getBalance(
        this.relayerRecipient
      );
      var recipientFeeAccount = await this.provider.connection.getBalance(
        this.recipientFee
      );

      console.log(
        `recipientFeeAccount ${new anchor.BN(
          recipientFeeAccount
        ).toString()} == ${new anchor.BN(this.recipientFeeBalancePriorLastTx)
          .sub(this.feeAmount)
          .toString()}`
      );

      console.log(
        `relayerAccount ${new anchor.BN(
          relayerAccount
        ).toString()} == ${new anchor.BN(
          this.relayerRecipientAccountBalancePriorLastTx
        )
          .sub(new anchor.BN(this.relayerFee))
          .toString()}`
      );
      assert(
        new anchor.BN(relayerAccount).toString() ==
          new anchor.BN(this.relayerRecipientAccountBalancePriorLastTx)
            .sub(new anchor.BN(this.relayerFee))
            .toString()
      );
    } else {
      throw Error('mode not supplied');
    }
  }
}

export async function getPdaAddresses({
  tx_integrity_hash,
  nullifiers,
  leftLeaves,
  merkleTreeProgram,
  verifierProgram,
} : {
  tx_integrity_hash: any;
  nullifiers: any[];
  leftLeaves: any[];
  merkleTreeProgram: anchor.Program;
  verifierProgram: anchor.Program;
}) {
  console.log('new Uint8Array(nullifier0) ', new Uint8Array(nullifiers[0]));
  
  let nullifierPdaPubkeys = [];
  for (var i in nullifiers) {
    nullifierPdaPubkeys.push(
      (
        await PublicKey.findProgramAddress(
          [
            Buffer.from(new Uint8Array(nullifiers[i])),
            anchor.utils.bytes.utf8.encode('nf'),
          ],
          merkleTreeProgram.programId
        )
      )[0]
    );
  }

  let leavesPdaPubkeys = [];
  for (var i in leftLeaves) {
    leavesPdaPubkeys.push(
      (
        await PublicKey.findProgramAddress(
          [
            Buffer.from(new Uint8Array(leftLeaves[i])),
            anchor.utils.bytes.utf8.encode('leaves'),
          ],
          merkleTreeProgram.programId
        )
      )[0]
    );
  }

  return {
    signerAuthorityPubkey: (
      await PublicKey.findProgramAddress(
        [merkleTreeProgram.programId.toBytes()],
        verifierProgram.programId
      )
    )[0],

    escrow: (
      await PublicKey.findProgramAddress(
        [anchor.utils.bytes.utf8.encode('escrow')],
        verifierProgram.programId
      )
    )[0],
    verifierStatePubkey: (
      await PublicKey.findProgramAddress(
        [
          Buffer.from(new Uint8Array(tx_integrity_hash)),
          anchor.utils.bytes.utf8.encode('storage'),
        ],
        verifierProgram.programId
      )
    )[0],
    feeEscrowStatePubkey: (
      await PublicKey.findProgramAddress(
        [
          Buffer.from(new Uint8Array(tx_integrity_hash)),
          anchor.utils.bytes.utf8.encode('escrow'),
        ],
        verifierProgram.programId
      )
    )[0],
    merkleTreeUpdateState: (
      await PublicKey.findProgramAddress(
        [
          Buffer.from(new Uint8Array(leftLeaves[0])),
          anchor.utils.bytes.utf8.encode('storage'),
        ],
        merkleTreeProgram.programId
      )
    )[0],
    nullifierPdaPubkeys,
    leavesPdaPubkeys,
    tokenAuthority: (
      await PublicKey.findProgramAddress(
        [anchor.utils.bytes.utf8.encode('spl')],
        merkleTreeProgram.programId
      )
    )[0],
  };
}
