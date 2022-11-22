const {U64, I64} = require('n64');
const anchor = require("@project-serum/anchor")
const nacl = require('tweetnacl')
const FIELD_SIZE = new anchor.BN('21888242871839275222246405745257275088548364400416034343698204186575808495617');
export const createEncryptionKeypair = () => nacl.box.keyPair()
var assert = require('assert');
let circomlibjs = require("circomlibjs")
var ffjavascript = require('ffjavascript');
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;
import { MerkleTreeProgram } from "../idls/merkle_tree_program";
import {toBufferLE} from 'bigint-buffer';

import {
  MERKLE_TREE_KEY,
  DEFAULT_PROGRAMS,
  ADMIN_AUTH_KEYPAIR,
  ADMIN_AUTH_KEY,
  MERKLE_TREE_SIZE,
  MERKLE_TREE_KP,
  MERKLE_TREE_SIGNER_AUTHORITY,
  PRIVATE_KEY,
  FIELD_SIZE,
  MINT_PRIVATE_KEY,
  MINT
  } from "./constants";
import {Connection, PublicKey, Keypair, SystemProgram, TransactionMessage, ComputeBudgetProgram,  AddressLookupTableAccount, VersionedTransaction, sendAndConfirmRawTransaction } from "@solana/web3.js";
import { newAccountWithLamports  } from "./test_transactions";
import { TOKEN_PROGRAM_ID, getAccount  } from '@solana/spl-token';
import {checkRentExemption} from './test_checks';
import {unpackLeavesAccount} from './unpack_accounts';
const newNonce = () => nacl.randomBytes(nacl.box.nonceLength);



// add verifier class which is passed in with the constructor
// this class replaces the send transaction, also configures path the provingkey and witness, the inputs for the integrity hash
// input custom verifier with three functions by default prepare, proof, send
// include functions from sdk in shieldedTransaction

// Changes for instantiation
// replace verifierProgram with verifier class
// remove merkleTreeProgram
export class shieldedTransaction {
  constructor({
    // keypair, // : Keypair shielded pool keypair that is derived from seedphrase. OutUtxo: supply pubkey
    encryptionKeypair = createEncryptionKeypair(),
    relayerFee = U64(10_000),
    merkleTreePubkey,
    merkleTree = null,
    merkleTreeAssetPubkey = null,
    recipient, //PublicKey
    lookupTable, //PublicKey
    payer, //: Keypair
    relayerPubkey = null, //PublicKey
    preInsertedLeavesIndex,
    provider,
    merkleTreeFeeAssetPubkey,
    relayerRecipient,
    registeredVerifierPda,
    poseidon,
    verifier,
    shuffleEnabled = true,
  }) {
      if (relayerPubkey == null) {
          this.relayerPubkey = new PublicKey(payer.publicKey);
      } else {
         this.relayerPubkey = relayerPubkey;
      }
      this.relayerRecipient = relayerRecipient;
      this.preInsertedLeavesIndex = preInsertedLeavesIndex;
      this.merkleTreeProgram = anchor.workspace.MerkleTreeProgram as Program<MerkleTreeProgram>;
      this.verifier = verifier;
      this.lookupTable = lookupTable;
      this.feeAsset = new anchor.BN(anchor.web3.SystemProgram.programId._bn.toString()).mod(FIELD_SIZE);
      this.relayerFee = relayerFee;
      // this.merkleTreeIndex = merkleTreeIndex;
      this.merkleTreePubkey = merkleTreePubkey;
      this.merkleTreeAssetPubkey = merkleTreeAssetPubkey;
      this.merkleTree = null;
      this.utxos = [];
      this.feeUtxos = [];
      this.encryptionKeypair = encryptionKeypair;
      this.payer = payer;

      this.provider = provider;
      this.recipient = recipient;
      this.merkleTreeFeeAssetPubkey = merkleTreeFeeAssetPubkey;
      this.registeredVerifierPda = registeredVerifierPda;
      this.merkleTree = merkleTree;
      this.poseidon = poseidon;
      this.sendTransaction = verifier.sendTransaction;
      this.shuffle = shuffleEnabled;
    }

    // async getMerkleTree() {
    //   this.poseidon = await circomlibjs.buildPoseidonOpt();
    //   if (this.keypair == null) {
    //     this.keypair = new light.Keypair(this.poseidon);
    //   }
    //   this.merkleTree = await light.buildMerkelTree(this.poseidon, 18, []);
    //   this.merkleTreeLeavesIndex = 0;
    //
    // }

    async getRootIndex() {
      let root = Uint8Array.from(leInt2Buff(unstringifyBigInts(this.merkleTree.root()), 32));
      let merkle_tree_account = await this.provider.connection.getAccountInfo(this.merkleTreePubkey);
      let merkle_tree_account_data  = this.merkleTreeProgram.account.merkleTree._coder.accounts.decode('MerkleTree', merkle_tree_account.data);

       merkle_tree_account_data.roots.map((x, index)=> {
        if (x.toString() === root.toString()) {
          this.root_index =  index;
        }
      })

    }
    prepareUtxos() {
        /// Validation
        if (this.inputUtxos.length > 10 || this.outputUtxos.length > 2) {
            throw new Error('Incorrect inputUtxos/outputUtxos count');
        }

        console.log("inputUtxos.length ", this.inputUtxos.length);
        /// fill inputUtxos until 2 or 10
        while (this.inputUtxos.length !== 2 && this.inputUtxos.length < 10) {
          this.inputUtxos.push(new utxos_1.default(poseidon));
          // throw "inputUtxos.length > 2 are not implemented";
        }

        /// if there are no outputUtxo add one
        while (this.outputUtxos.length < 2) {
          this.outputUtxos.push(new utxos_1.default(poseidon));
        }
        /// mixes the input utxos
        /// mixes the output utxos
        if (this.shuffle) {
          console.log("shuffling utxos")

          this.inputUtxos = shuffle(this.inputUtxos);
          this.outputUtxos = shuffle(this.outputUtxos);

        } else {
          console.log("commented shuffle")
        }


        /// the fee plus the amount to pay has to be bigger than the amount in the input utxo
        // which doesn't make sense it should be the other way arround right
        // the external amount can only be made up of utxos of asset[0]

        // This might be too specific since the circuit allows assets to be in any index
        const getExternalAmount (assetIndex) => {
          return new anchor.BN(0)
              .add(this.outputUtxos.filter((utxo) => {return utxo.assets[assetIndex] == this.assetPubkeys[assetIndex]}).reduce((sum, utxo) => (
                // add all utxos of the same asset
                sum.add(utxo.amounts[assetIndex])
              ), new anchor.BN(0)))
              .sub(this.inputUtxos.filter((utxo) => {return utxo.assets[assetIndex] == this.assetPubkeys[assetIndex]}).reduce((sum, utxo) =>
                sum.add(utxo.amounts[assetIndex]),
                new anchor.BN(0)
            ));
        }

        this.externalAmountBigNumber = getExternalAmount(1)

        this.feeAmount =  getExternalAmount(0);

        /// if it is a deposit and the amount going in is smaller than 0 throw error
        if (this.action === 'DEPOSIT' &&
            this.externalAmountBigNumber < new anchor.BN(0)) {
            throw new Error(`Incorrect Extamount: ${this.externalAmountBigNumber.toNumber()}`);
        }

        this.outputUtxos.map((utxo) => {
          if (utxo.assets == null) {
            throw new Error(`output utxo asset not defined ${utxo}`);
          }
        });

        this.inputUtxos.map((utxo) => {
          if (utxo.assets == null) {
            throw new Error(`intput utxo asset not defined ${utxo}`);
          }
        });

        let assetPubkeys = [feeAsset,assets].concat();
        if (assets.length != 3) {
          throw new Error(`assetPubkeys.length != 3 ${assets}`);
        }

        if (assets[0] === assets[1] || assets[1] === assets[2] || assets[0] === assets[2]) {
          throw new Error(`asset pubKeys need to be distinct ${assets}`);
        }

        const getIndices = (utxos) => {
          return utxos.map((utxo) => {
            let tmpInIndices = []
            for (var a = 0; a < 3; a++) {
              let tmpInIndices1 = []
                for (var i = 0; i < utxo.assets.length; i++) {
                  if (utxo.assets[i] === assets[a]) {
                    tmpInIndices1.push("1")
                  } else {
                    tmpInIndices1.push("0")
                  }
                }
                tmpInIndices.push(tmpInIndices1)
            }
            inIndices.push(tmpInIndices)
          });
        };

        this.inIndices = getIndices(this.inputUtxos);
        this.outIndices = getIndices(this.outputUtxos);
        console.log("inIndices: ", inIndices)
        console.log("outIndices: ", outIndices)
    };

    prepareTransaction (
      // inputUtxos = [],
      // outputUtxos = [],
      // merkelTree,
      // merkleTreeIndex,
      // merkleTreePubkeyBytes,
      // externalAmountBigNumber,
      // relayerFee,
      // recipient,
      // relayer,
      // action,
      // encryptionKeypair,
      // inIndices,
      // outIndices,
      // assetPubkeys,
      // mintPubkey,
      // test,
      // feeAmount,
      // recipientFee
    ) {
          /// mixes the input utxos
          /// mixes the output utxos
          // inputUtxos = (0, shuffle_1.shuffle)(inputUtxos);
          // outputUtxos = (0, shuffle_1.shuffle)(outputUtxos);
          // console.log(`input utxos -> `, inputUtxos)
          // console.log(`outputUtxos -> `, outputUtxos)
          // console.log(`merkelTree -> `, merkelTree)
          // console.log(`relayerFee -> `, relayerFee)
          // console.log(`recipient -> `, recipient)
          // console.log(`Action[action] -> `, Action[action])
          // console.log(`encryptionKeypair -> `, encryptionKeypair)
          let inputMerklePathIndices = [];
          let inputMerklePathElements = [];
          /// if the input utxo has an amount bigger than 0 and it has an valid index add it to the indices of the merkel tree
          /// also push the path to the leaf
          /// else push a 0 to the indices
          /// and fill the path to the leaf with 0s

          // getting merkle proofs
          for (const inputUtxo of this.inputUtxos) {
              if (this.test) {
                inputMerklePathIndices.push(0);
                inputMerklePathElements.push(new Array(this.merkelTree.levels).fill(0));
              }

              else if (this.inputUtxo.amounts[0] > 0 || this.inputUtxo.amounts[1] > 0|| this.inputUtxo.amounts[2] > 0)  {
                  this.inputUtxo.index = this.merkelTree.indexOf(this.inputUtxo.getCommitment());
                  if (this.inputUtxo.index || this.inputUtxo.index == 0) {
                      if (this.inputUtxo.index < 0) {
                          throw new Error(`Input commitment ${this.inputUtxo.getCommitment()} was not found`);
                      }
                      inputMerklePathIndices.push(this.inputUtxo.index);
                      inputMerklePathElements.push(this.merkelTree.path(this.inputUtxo.index).pathElements);
                  }
              }

              else {
                  inputMerklePathIndices.push(0);
                  inputMerklePathElements.push(new Array(this.merkelTree.levels).fill(0));
              }
          }

          let relayer_fee
          if (this.action !== 'DEPOSIT') {
              relayer_fee = toBufferLE(new anchor.BN(this.relayerFee.toString()), 8);
          }
          else {
              relayer_fee = new Uint8Array(8).fill(0);
          }
          console.log("feesLE: ", relayer_fee);

          // ----------------------- getting integrity hash -------------------
          const nonces = [newNonce(), newNonce()];
          // const senderThrowAwayKeypairs = [
          //     newKeypair(),
          //     newKeypair()
          // ];
          // console.log(outputUtxos)
          /// Encrypt outputUtxos to bytes
          // removed throwaway keypairs since we already have message integrity with integrity_hashes
          // TODO: should be a hardcoded keypair in production not the one of the sender
          let encryptedOutputs = [ ];
          outputUtxos.map((utxo, index) => encryptedOutputs.push(utxo.encrypt(nonces[index], encryptionKeypair, encryptionKeypair)));

          // console.log("removed senderThrowAwayKeypairs TODO: always use fixed keypair or switch to salsa20 without poly153");
          let encryptedUtxos = new Uint8Array([...encryptedOutputs[0], ...nonces[0], ...encryptedOutputs[1], ...nonces[1], ...new Array(256 - 174).fill(0)]);

          const extData = {
              recipient: this.recipient.toBytes(),
              recipientFee: this.recipientFee.toBytes(),
              relayer:      this.payer.publicKey.toBytes(),
              relayer_fee: feesLE,
              merkleTreePubkeyBytes: merkleTreePubkeyBytes
          };
          // const { extDataHash, extDataBytes } = (0, getExternalDataHash_1.getExtDataHash)(extData.recipient, extData.recipientFee, extData.relayer, extData.relayer_fee,merkleTreeIndex, encryptedUtxos);
          let extDataBytes = new Uint8Array([
              ...this.recipient.toBytes(),
              ...this.recipientFee.toBytes(),
              ...this.payer.publicKey.toBytes(),
              ...relayer_fee,
              ...encryptedUtxos
          ]);
          const hash = ethers.ethers.utils.keccak256(Buffer.from(extDataBytes));
          // const hash = anchor.utils.sha256.hash(extDataBytes)
          console.log("Hash: ", hash);
          this.extDataHash = ethers.BigNumber.from(hash).mod(FIELD_SIZE), //new anchor.BN(anchor.utils.bytes.hex.decode(hash)).mod(constants_1.FIELD_SIZE),

          // ----------------------- building input object -------------------
          this.input = {
              root: merkelTree.root(),
              inputNullifier: this.inputUtxos.map((x) => x.getNullifier()),
              outputCommitment: this.outputUtxos.map((x) => x.getCommitment()),
              // TODO: move public and fee amounts into tx preparation
              publicAmount: this.externalAmountBigNumber
                  .add(constants_1.FIELD_SIZE)
                  .mod(constants_1.FIELD_SIZE)
                  .toString(),
              extDataHash: extDataHash.toString(),
              feeAmount: new anchor.BN(feeAmount)
                  .add(constants_1.FIELD_SIZE)
                  .mod(constants_1.FIELD_SIZE)
                  .toString(),
              mintPubkey,
              // data for 2 transaction inputUtxos
              inAmount: this.inputUtxos.map((x) => x.amounts),
              inPrivateKey: this.inputUtxos.map((x) => x.keypair.privkey),
              inBlinding: this.inputUtxos.map((x) => x.blinding),
              inPathIndices: inputMerklePathIndices,
              inPathElements: inputMerklePathElements,
              assetPubkeys,
              // data for 2 transaction outputUtxos
              outAmount: this.outputUtxos.map((x) => x.amounts),
              outBlinding: this.outputUtxos.map((x) => x.blinding),
              outPubkey: this.outputUtxos.map((x) => x.keypair.pubkey),
              inIndices: this.inIndices,
              outIndices: this.outIndices,
              inInstructionType: inputUtxos.map((x) => x.instructionType),
              outInstructionType: outputUtxos.map((x) => x.instructionType)
          };
          // console.log("extDataHash: ", input.extDataHash);
          // console.log("input.inputNullifier ",input.inputNullifier[0] );
          // console.log("input feeAmount: ", input.feeAmount);
          // console.log("input publicAmount: ", input.publicAmount);
          // console.log("input relayerFee: ", relayerFee);
          //
          // console.log("inIndices ", JSON.stringify(inIndices, null, 4));
          // console.log("outIndices ", JSON.stringify(outIndices, null, 4));

          // return {
          //         extAmount: extData.extAmount,
          //         externalAmountBigNumber,
          //         extDataBytes,
          //         encryptedUtxos,
          //         input,
          //         relayerFee
          //     };
    }

    async prepareTransactionFull({
      inputUtxos,
      outputUtxos,
      action,
      assetPubkeys,
      recipient,
      mintPubkey = 0,
      relayerFee = null, // public amount of the fee utxo adjustable if you want to deposit a fee utxo alongside your spl deposit
      shuffle = true,
      recipientFee,
      sender
    }) {
      mintPubkey = assetPubkeys[1];
      if (assetPubkeys[0].toString() != this.feeAsset.toString()) {
        throw "feeAsset should be assetPubkeys[0]";
      }
      if (action == "DEPOSIT") {
        console.log("Deposit");

        this.relayerFee = relayerFee;
        this.sender = sender;
        this.senderFee  = new PublicKey(this.payer.publicKey);
        this.recipient = this.merkleTreeAssetPubkey;
        this.recipientFee = this.merkleTreeFeeAssetPubkey;

        if (this.relayerPubkey.toBase58() != new PublicKey(this.payer.publicKey).toBase58()) {
          throw "relayerPubkey and payer pubkey need to be equivalent at deposit";
        }
      } else if (action == "WITHDRAWAL") {
        this.senderFee = this.merkleTreeFeeAssetPubkey;
        this.recipientFee = recipientFee;
        this.sender = this.merkleTreeAssetPubkey;
        this.recipient = recipient;
        if (relayerFee != null) {
          this.relayerFee = relayerFee;
          if (relayerFee == undefined) {
            throw "relayerFee undefined";
          }
        }

      if (recipient == undefined) {
        throw "recipient undefined";
      }
      if (recipientFee == undefined) {
        throw "recipientFee undefined";
      }
    }

    this.assetPubkeys = assetPubkeys;
    this.mintPubkey = mintPubkey;
    this.action = action;

    this.prepareUtxos();
      // let res = light.prepareUtxos(
      //     inputUtxos,
      //     outputUtxos,
      //     this.relayerFee,
      //     this.assetPubkeys,
      //     this.action,
      //     this.poseidon,
      //     shuffle
      // );

      // this.inputUtxos = res.inputUtxos;
      // this.outputUtxos = res.outputUtxos;
      // this.inIndices = res.inIndices;
      // this.outIndices = res.outIndices;
      // this.externalAmountBigNumber = res.externalAmountBigNumber;
      // this.feeAmount = res.feeAmount;

    await this.prepareTransaction();
    await this.getRootIndex();
     //  let data = await light.prepareTransaction(
     //   this.inputUtxos,
     //   this.outputUtxos,
     //   this.merkleTree,
     //   this.merkleTreeIndex,
     //   this.merkleTreePubkey.toBytes(),
     //   this.externalAmountBigNumber,
     //   this.relayerFee,
     //   this.recipient,
     //   this.relayerPubkey,
     //   this.action,
     //   this.encryptionKeypair,
     //   this.inIndices,
     //   this.outIndices,
     //   this.assetPubkeys,
     //   this.mintPubkey,
     //   false,
     //   this.feeAmount,
     //   this.recipientFee
     // )
     // this.input = data.input;
     assert(this.input.mintPubkey == this.mintPubkey);
     assert(this.input.mintPubkey == this.assetPubkeys[1]);
     // this.extAmount = data.extAmount;
     // this.externalAmountBigNumber = data.externalAmountBigNumber;
     // this.extDataBytes = data.extDataBytes;
     // this.encrypedUtxos = data.encryptedUtxos
     if (this.externalAmountBigNumber != 0) {
       if (assetPubkeys[1].toString() != mintPubkey.toString()) {
         throw "mintPubkey should be assetPubkeys[1]";
       }
     }
    }

    async proof() {
      if (this.merkleTree == null) {
        throw "merkle tree not built";
      }
      if (this.inIndices == null) {
        throw "transaction not prepared";
      }


      const buffer = readFileSync(`${this.verifier.wtnsGenPath}.wasm`);

      let witnessCalculator =  await calculateWtns(buffer)
      console.time('Proof generation');
      wtns= await witnessCalculator.calculateWTNSBin(stringifyBigInts(this.input),0);

      const { proof, publicSignals } = await snarkjs.groth16.prove(`${this.verifier.zkeyPath}.zkey`, wtns);
      this.proofJson = JSON.stringify(proof, null, 1);
      this.publicInputsJson = JSON.stringify(publicSignals, null, 1);
      console.timeEnd('Proof generation');

      const vKey = await snarkjs.zKey.exportVerificationKey(`${this.verifier.zkeyPath}.zkey`);
      const res = await snarkjs.groth16.verify(vKey, publicSignals, proof);
      if (res === true) {
          console.log('Verification OK');
      }
      else {
          console.log('Invalid proof');
          throw new Error('Invalid Proof');
      }

      this.publicInputsBytes = JSON.parse(publicInputsJson.toString());
      for (var i in this.publicInputsBytes) {
          this.publicInputsBytes[i] = Array.from(leInt2Buff(unstringifyBigInts(this.publicInputsBytes[i]), 32)).reverse();
      }


      // return {
      //         extAmount: extAmount,
      //         externalAmountBigNumber,
      //         extDataBytes,
      //         publicInputs,//
      //         proofBytes: yield (0, parseProofToBytesArray_1.parseProofToBytesArray)(proofJson),
      //         encryptedOutputs: encryptedOutputs,
      //         relayerFee
      //     };

      this.proofBytes = parseProofToBytesArray(proofJson),


      this.proofData = proofData;
      await this.getPdaAddresses()
      return this.proofData;
    }

    async getPdaAddresses() {
      let tx_integrity_hash = this.proofData.publicInputs.txIntegrityHash;
      let nullifiers = this.proofData.publicInputs.nullifiers;
      let leftLeaves = [this.proofData.publicInputs.leaves[0]];
      let merkleTreeProgram = this.merkleTreeProgram;
      let signer = this.payer.publicKey;

      let nullifierPdaPubkeys = [];
      for (var i in nullifiers) {
        nullifierPdaPubkeys.push(
        (await PublicKey.findProgramAddress(
            [Buffer.from(new Uint8Array(nullifiers[i])), anchor.utils.bytes.utf8.encode("nf")],
            merkleTreeProgram.programId))[0]);
      }

      let leavesPdaPubkeys = [];
      for (var i in leftLeaves) {
        leavesPdaPubkeys.push(
        (await PublicKey.findProgramAddress(
            [Buffer.from(Array.from(leftLeaves[i]).reverse()), anchor.utils.bytes.utf8.encode("leaves")],
            merkleTreeProgram.programId))[0]);
      }

      let pdas = {
        signerAuthorityPubkey: (await PublicKey.findProgramAddress(
            [merkleTreeProgram.programId.toBytes()],
            this.verifier.verifierProgram.programId))[0],

        escrow: (await PublicKey.findProgramAddress(
            [anchor.utils.bytes.utf8.encode("escrow")],
            this.verifier.verifierProgram.programId))[0],
        verifierStatePubkey: (await PublicKey.findProgramAddress(
            [signer.toBytes(), anchor.utils.bytes.utf8.encode("VERIFIER_STATE")],
            this.verifier.verifierProgram.programId))[0],
        feeEscrowStatePubkey: (await PublicKey.findProgramAddress(
            [Buffer.from(new Uint8Array(tx_integrity_hash)), anchor.utils.bytes.utf8.encode("escrow")],
            this.verifier.verifierProgram.programId))[0],
        merkleTreeUpdateState: (await PublicKey.findProgramAddress(
            [Buffer.from(new Uint8Array(leftLeaves[0])), anchor.utils.bytes.utf8.encode("storage")],
            merkleTreeProgram.programId))[0],
        nullifierPdaPubkeys,
        leavesPdaPubkeys,
        tokenAuthority: (await PublicKey.findProgramAddress(
            [anchor.utils.bytes.utf8.encode("spl")],
            merkleTreeProgram.programId
          ))[0],
      };
      this.escrow = pdas.escrow;
      this.leavesPdaPubkeys = pdas.leavesPdaPubkeys;
      this.nullifierPdaPubkeys = pdas.nullifierPdaPubkeys;
      this.signerAuthorityPubkey = pdas.signerAuthorityPubkey;
      this.tokenAuthority = pdas.tokenAuthority;
      this.verifierStatePubkey = pdas.verifierStatePubkey;
    }

    async checkBalances(){
      // Checking that nullifiers were inserted
      this.is_token = true;

      for (var i in this.nullifierPdaPubkeys) {

        var nullifierAccount = await this.provider.connection.getAccountInfo(
          this.nullifierPdaPubkeys[i],
          {
          commitment: 'confirmed',
          preflightCommitment: 'confirmed',
        }
        );

        await checkRentExemption({
          account: nullifierAccount,
          connection: this.provider.connection
        });
      }
      let leavesAccount
      var leavesAccountData
      // Checking that leaves were inserted
      for (var i in this.leavesPdaPubkeys) {

        leavesAccountData = await this.merkleTreeProgram.account.twoLeavesBytesPda.fetch(
          this.leavesPdaPubkeys[i]
        );

        try {

          assert(leavesAccountData.nodeLeft.toString() === this.proofData.publicInputs.leaves[0].reverse().toString(), "left leaf not inserted correctly")
          assert(leavesAccountData.nodeRight.toString() === this.proofData.publicInputs.leaves[1].reverse().toString(), "right leaf not inserted correctly")
          assert(leavesAccountData.merkleTreePubkey.toBase58() === this.merkleTreePubkey.toBase58(), "merkleTreePubkey not inserted correctly")
          for (var i in this.encrypedUtxos) {

            if (leavesAccountData.encryptedUtxos[i] !== this.encrypedUtxos[i]) {
              console.log(i);
            }
            assert(leavesAccountData.encryptedUtxos[i] === this.encrypedUtxos[i], "encryptedUtxos not inserted correctly");
          }

        } catch(e) {
          console.log("leaves: ", e);
        }
      }

      console.log(`mode ${this.action}, this.is_token ${this.is_token}`);

      try {
        console.log("this.preInsertedLeavesIndex ", this.preInsertedLeavesIndex);

        var preInsertedLeavesIndexAccount = await this.provider.connection.getAccountInfo(
          this.preInsertedLeavesIndex
        )

        console.log(preInsertedLeavesIndexAccount);
        const preInsertedLeavesIndexAccountAfterUpdate = this.merkleTreeProgram.account.preInsertedLeavesIndex._coder.accounts.decode('PreInsertedLeavesIndex', preInsertedLeavesIndexAccount.data);
        console.log("Number(preInsertedLeavesIndexAccountAfterUpdate.nextIndex) ", Number(preInsertedLeavesIndexAccountAfterUpdate.nextIndex));
        console.log(`${Number(leavesAccountData.leftLeafIndex) } + ${this.leavesPdaPubkeys.length * 2}`);

        assert(Number(preInsertedLeavesIndexAccountAfterUpdate.nextIndex) == Number(leavesAccountData.leftLeafIndex) + this.leavesPdaPubkeys.length * 2)

      } catch(e) {
        console.log("preInsertedLeavesIndex: ", e);

      }

      if (this.action == "DEPOSIT" && this.is_token == false) {
        var recipientAccount = await this.provider.connection.getAccountInfo(this.recipient)
        assert(recipientAccount.lamports == (I64(this.recipientBalancePriorTx).add(this.proofData.externalAmountBigNumber.toString())).toString(), "amount not transferred correctly");

      } else if (this.action == "DEPOSIT" && this.is_token == true) {
        console.log("DEPOSIT and token");
        console.log("this.recipient: ", this.recipient);

          var recipientAccount = await getAccount(
          this.provider.connection,
          this.recipient,
          TOKEN_PROGRAM_ID
        );
        var recipientFeeAccountBalance = await this.provider.connection.getBalance(this.recipientFee);

        // console.log(`Balance now ${senderAccount.amount} balance beginning ${senderAccountBalancePriorLastTx}`)
        // assert(senderAccount.lamports == (I64(senderAccountBalancePriorLastTx) - I64.readLE(this.proofData.extAmount, 0)).toString(), "amount not transferred correctly");

        console.log(`Balance now ${recipientAccount.amount} balance beginning ${this.recipientBalancePriorTx}`)
        console.log(`Balance now ${recipientAccount.amount} balance beginning ${(Number(this.recipientBalancePriorTx) + Number(this.proofData.externalAmountBigNumber))}`)
        assert(recipientAccount.amount == (Number(this.recipientBalancePriorTx) + Number(this.proofData.externalAmountBigNumber)).toString(), "amount not transferred correctly");
        console.log(`Blanace now ${recipientFeeAccountBalance} ${Number(this.recipientFeeBalancePriorTx) + Number(this.feeAmount)}`);
        console.log("fee amount: ", this.feeAmount);
        console.log("fee amount from inputs. ", new anchor.BN(this.proofData.publicInputs.feeAmount.slice(24,32)).toString());
        console.log("pub amount from inputs. ", new anchor.BN(this.proofData.publicInputs.publicAmount.slice(24,32)).toString());

        console.log("recipientFeeBalancePriorTx: ", this.recipientFeeBalancePriorTx);

        var senderFeeAccountBalance = await this.provider.connection.getBalance(this.senderFee);
        console.log("senderFeeAccountBalance: ", senderFeeAccountBalance);
        console.log("this.senderFeeBalancePriorTx: ", this.senderFeeBalancePriorTx);

        assert(recipientFeeAccountBalance == Number(this.recipientFeeBalancePriorTx) + Number(this.feeAmount));
        console.log(`${Number(this.senderFeeBalancePriorTx)} - ${Number(this.feeAmount)} == ${senderFeeAccountBalance}`);
        assert(Number(this.senderFeeBalancePriorTx) - Number(this.feeAmount) - 5000 == Number(senderFeeAccountBalance) );

      } else if (this.action == "WITHDRAWAL" && this.is_token == false) {
        var senderAccount = await this.provider.connection.getAccountInfo(this.sender)
        var recipientAccount = await this.provider.connection.getAccountInfo(this.recipient)
        // console.log("senderAccount.lamports: ", senderAccount.lamports)
        // console.log("I64(senderAccountBalancePriorLastTx): ", I64(senderAccountBalancePriorLastTx).toString())
        // console.log("Sum: ", ((I64(senderAccountBalancePriorLastTx).add(I64.readLE(this.proofData.extAmount, 0))).sub(I64(relayerFee))).toString())

        assert(senderAccount.lamports == ((I64(senderAccountBalancePriorLastTx).add(I64.readLE(this.proofData.extAmount, 0))).sub(I64(relayerFee))).toString(), "amount not transferred correctly");

        var recipientAccount = await this.provider.connection.getAccountInfo(recipient)
        // console.log(`recipientAccount.lamports: ${recipientAccount.lamports} == sum ${((I64(Number(this.recipientBalancePriorTx)).sub(I64.readLE(this.proofData.extAmount, 0))).add(I64(relayerFee))).toString()}

        assert(recipientAccount.lamports == ((I64(Number(this.recipientBalancePriorTx)).sub(I64.readLE(this.proofData.extAmount, 0)))).toString(), "amount not transferred correctly");


      }  else if (this.action == "WITHDRAWAL" && this.is_token == true) {
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


        // assert(senderAccount.amount == ((I64(Number(senderAccountBalancePriorLastTx)).add(I64.readLE(this.proofData.extAmount, 0))).sub(I64(relayerFee))).toString(), "amount not transferred correctly");
        console.log(`${recipientAccount.amount}, ${((new anchor.BN(this.recipientBalancePriorTx)).sub(this.proofData.externalAmountBigNumber)).toString()}`)
        assert(recipientAccount.amount.toString() == ((new anchor.BN(this.recipientBalancePriorTx)).sub(this.proofData.externalAmountBigNumber)).toString(), "amount not transferred correctly");



        var relayerAccount = await this.provider.connection.getBalance(this.relayerRecipient);
        var recipientFeeAccount = await this.provider.connection.getBalance(this.recipientFee);
        console.log("recipientFeeAccount ", recipientFeeAccount);
        console.log("this.feeAmount: ", this.feeAmount);
        console.log("recipientFeeBalancePriorTx ", this.recipientFeeBalancePriorTx);
        console.log(`recipientFeeAccount ${(new anchor.BN(recipientFeeAccount).add(new anchor.BN(this.relayerFee.toString()))).add(new anchor.BN("5000")).toString()} == ${new anchor.BN(this.recipientFeeBalancePriorTx).sub(new anchor.BN(this.feeAmount)).toString()}`)

        console.log("relayerAccount ", relayerAccount);
        console.log("this.relayerFee: ", this.relayerFee);
        console.log("relayerRecipientAccountBalancePriorLastTx ", this.relayerRecipientAccountBalancePriorLastTx);
        console.log(`relayerFeeAccount ${new anchor.BN(relayerAccount).sub(new anchor.BN(this.relayerFee.toString())).toString()} == ${new anchor.BN(this.relayerRecipientAccountBalancePriorLastTx)}`)

        // console.log(`relayerAccount ${new anchor.BN(relayerAccount).toString()} == ${new anchor.BN(this.relayerRecipientAccountBalancePriorLastTx).sub(new anchor.BN(this.relayerFee)).toString()}`)
        assert((new anchor.BN(recipientFeeAccount).add(new anchor.BN(this.relayerFee.toString()))).toString() == new anchor.BN(this.recipientFeeBalancePriorTx).sub(new anchor.BN(this.feeAmount)).toString());
        assert(new anchor.BN(relayerAccount).sub(new anchor.BN(this.relayerFee.toString())).add(new anchor.BN("5000")).toString() == new anchor.BN(this.relayerRecipientAccountBalancePriorLastTx).toString());



      } else {
        throw Error("mode not supplied");
      }
    }

}

const shuffle = function (utxos: Utxo[]) {
  let currentIndex: number = utxos.length
  let randomIndex: number

  // While there remain elements to shuffle...
  while (0 !== currentIndex) {
    // Pick a remaining element...
    randomIndex = Math.floor(Math.random() * currentIndex)
    currentIndex--

    // And swap it with the current element.
    ;[utxos[currentIndex], utxos[randomIndex]] = [
      utxos[randomIndex],
      utxos[currentIndex],
    ]
  }

  return utxos
}

export const parseProofToBytesArray = async function (data: any) {
  var mydata = JSON.parse(data.toString())

  for (var i in mydata) {
    if (i == 'pi_a') {
      for (var j in mydata[i]) {
        mydata[i][j] = leInt2Buff(
          unstringifyBigInts(mydata[i][j]),
          32, // 48
        ).reverse()
      }
    } else if (i == 'pi_b') {
      for (var j in mydata[i]) {
        for (var z in mydata[i][j]) {
          mydata[i][j][z] = leInt2Buff(
            unstringifyBigInts(mydata[i][j][z]),
            32, // 48
          )
        }
      }
    } else if (i == 'pi_c') {
      for (var j in mydata[i]) {
        mydata[i][j] = leInt2Buff(
          unstringifyBigInts(mydata[i][j]),
          32, //48
        ).reverse()
      }
    }
  }
  let mydataStripped = [
    mydata.pi_a[0],
    mydata.pi_a[1],
    Array.from([].concat.apply([], mydata.pi_b[0])).reverse(),
    Array.from([].concat.apply([], mydata.pi_b[1])).reverse(),
    mydata.pi_c[0],
    mydata.pi_c[1],
  ]
  // TODO: replace x with Array.flat()
  var merged = [].concat.apply([], mydataStripped)
  let x: any = []
  merged.map((array: any) => {
    array.map((byte: any) => {
      x.push(byte)
    })
  })

  return x
}
