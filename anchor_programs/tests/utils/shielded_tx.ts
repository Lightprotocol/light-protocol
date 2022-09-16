const light = require('../../light-protocol-sdk');
const {U64, I64} = require('n64');
const anchor = require("@project-serum/anchor")
const nacl = require('tweetnacl')
const FIELD_SIZE = new anchor.BN('21888242871839275222246405745257275088548364400416034343698204186575808495617');
export const createEncryptionKeypair = () => nacl.box.keyPair()
var assert = require('assert');



export class shieldedTransaction {
  constructor({
    assetPubkeys = null,
    keypair = null, // shielded pool keypair that is derived from seedphrase. OutUtxo: supply pubkey
    encryptionKeypair = createEncryptionKeypair(),
    feeAsset = new anchor.BN(anchor.web3.SystemProgram.programId._bn.toString()).mod(FIELD_SIZE),
    relayerFee = U64(10_000),
    merkleTreeIndex = 0,
    merkleTreePubkey = null,
    merkleTree = null,
    merkleTreeAssetPubkey = null,
    relayerPubkey = null,
    poseidon = null
  }) {
      if (keypair == null) {
        keypair = new light.Keypair(poseidon);
      } else {
        this.keypair = keypair;

      }
      this.feeAsset = feeAsset;
      this.relayerFee = relayerFee;
      this.merkleTreeIndex = merkleTreeIndex;
      this.merkleTreePubkey = merkleTreePubkey;
      this.merkleTreeAssetPubkey = merkleTreeAssetPubkey;
      this.merkleTree = null;
      this.relayerPubkey = relayerPubkey;
      this.utxos = [];
      this.feeUtxos = [];
      this.encryptionKeypair = encryptionKeypair;
      this.poseidon = poseidon;
    }

    async getMerkleTree() {
      this.merkleTree = await light.buildMerkelTree(this.poseidon);
      this.merkleTreeLeavesIndex = 0;
    }

    prepareUtxos({
      inputUtxos,
      outputUtxos,
      action,
      assetPubkeys,
      recipient = "MERKLE_TREE_PDA_TOKEN_USDC",
      mintPubkey = 0,
      relayerFee, // public amount of the fee utxo adjustable if you want to deposit a fee utxo alongside your spl deposit
      shuffle = true,
    }) {
      // mintPubkey = assetPubkeys[1];
      // if (assetPubkeys[1].toString() != mintPubkey.toString()) {
      //   throw "mintPubkey should be assetPubkeys[1]";
      // }
      if (assetPubkeys[0].toString() != this.feeAsset.toString()) {
        throw "feeAsset should be assetPubkeys[0]";
      }
      if (action == "DEPOSIT") {
        this.relayerFee = relayerFee;
      }
      this.recipient = recipient
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

      this.inputUtxos = res.inputUtxos;
      this.outputUtxos = res.outputUtxos;
      this.inIndices = res.inIndices;
      this.outIndices = res.outIndices;
      this.externalAmountBigNumber = res.externalAmountBigNumber;
      if (this.externalAmountBigNumber != 0) {
        if (assetPubkeys[1].toString() != mintPubkey.toString()) {
          throw "mintPubkey should be assetPubkeys[1]";
        }
      }
      // console.log("this.inputUtxos[0]: ", this.inputUtxos[0])
      // console.log("this.inputUtxos[1]: ", this.inputUtxos[1])
      // console.log("this.inputUtxos[2]: ", this.inputUtxos[2])
      // console.log("this.inputUtxos[3]: ", this.inputUtxos[3])
      //
      // console.log("this.outputUtxos[0]: ", this.outputUtxos[0])
      // console.log("this.outputUtxos[1]: ", this.outputUtxos[1])
      // console.log("this.outputUtxos[2]: ", this.outputUtxos[2])
      // console.log("this.outputUtxos[3]: ", this.outputUtxos[3])
      //
      // console.log("this.inIndices: ", this.inIndices)
      // console.log("this.outIndices: ", this.outIndices)
      // console.log("this.externalAmountBigNumber: ", this.externalAmountBigNumber)


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
       this.merkleTreeAssetPubkey,
       this.relayerPubkey,
       this.action,
       this.encryptionKeypair,
       this.inIndices,
       this.outIndices,
       this.assetPubkeys,
       this.mintPubkey,
       false,
       this.feeAmount
     )
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
      recipient = "MERKLE_TREE_PDA_TOKEN_USDC",
      mintPubkey = 0,
      relayerFee, // public amount of the fee utxo adjustable if you want to deposit a fee utxo alongside your spl deposit
      shuffle = true,
    }) {
      mintPubkey = assetPubkeys[1];
      if (assetPubkeys[0].toString() != this.feeAsset.toString()) {
        throw "feeAsset should be assetPubkeys[0]";
      }
      if (action == "DEPOSIT") {
        this.relayerFee = relayerFee;
      }
      this.recipient = recipient
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
       this.merkleTreeAssetPubkey,
       this.relayerPubkey,
       this.action,
       this.encryptionKeypair,
       this.inIndices,
       this.outIndices,
       this.assetPubkeys,
       this.mintPubkey,
       false,
       this.feeAmount
     )
     this.input = data.input;
     assert(this.input.mintPubkey == this.mintPubkey);
     assert(this.input.mintPubkey == this.assetPubkeys[1]);
     console.log("this.input.inIndices: ", this.input.inIndices);
     console.log(`this.input.mintPubkey ${this.input.mintPubkey}== this.mintPubkey${this.mintPubkey}`);
     this.extAmount = data.extAmount;
     this.externalAmountBigNumber = data.externalAmountBigNumber;
     this.extDataBytes = data.extDataBytes;
     this.encryptedOutputs = data.extDataBytes;
     if (this.externalAmountBigNumber != 0) {
       if (assetPubkeys[1].toString() != mintPubkey.toString()) {
         throw "mintPubkey should be assetPubkeys[1]";
       }
     }
    }

    async proof(insert) {
      if (this.merkleTree == null) {
        throw "merkle tree not built";
      }
      if (this.inIndices == null) {
        throw "transaction not prepared";
      }
      let proofData = await light.getProofMasp(
        this.input,
        this.extAmount,
        this.externalAmountBigNumber,
        this.extDataBytes,
        this.encryptedOutputs
      )

      this.outputUtxos.map((utxo) => {
        if (utxo.amounts[1] != 0 && utxo.assets[1] != this.feeAsset) {
            this.utxos.push(utxo)
        }
        if (utxo.amounts[0] != 0 && utxo.assets[0].toString() == this.feeAsset.toString()) {
          this.feeUtxos.push(utxo)
        }
      })
      this.inIndices = null;
      this.relayerFee = U64(10_000);
      // inserting output utxos into merkle tree
      if (insert != "NOINSERT") {
        for (var i = 0; i<this.outputUtxos.length; i++) {
          this.merkleTree.update(this.merkleTreeLeavesIndex, this.outputUtxos[i].getCommitment())
          this.merkleTreeLeavesIndex++;
        }

      }
      return proofData;
    }
}
