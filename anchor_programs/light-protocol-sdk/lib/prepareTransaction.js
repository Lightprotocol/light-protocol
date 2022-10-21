"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.prepareUtxos = void 0;
// prepare transaction is in proof.js

const enums_1 = require("./enums");
const utxos_1 = __importDefault(require("./utxos"));
const anchor = require("@project-serum/anchor")
const keypair_1 = require("./utils/keypair");
const constants_1 = require("./constants");
const shuffle_1 = require("./utils/shuffle");

const feeAsset = new anchor.BN(anchor.web3.SystemProgram.programId._bn.toString()).mod(constants_1.FIELD_SIZE)
// prepare transaction is in proof.js
const prepareUtxos = (
    inputUtxos = [],
    outputUtxos = [],
    relayerFee,
    assets = [],
    action,
    poseidon,
    shuffle = true,
    // swapConstraints,
    // swapRecipientPubkey,
    // swapSetupKeypair = new keypair_1.Keypair()
  ) => {
    /// Validation
    if (inputUtxos.length > 16 || outputUtxos.length > 2) {
        throw new Error('Incorrect inputUtxos/outputUtxos count');
    }
    // generates an setup utxo
    //
    /*
    if (action == "SWAP_SETUP") {
      swapConstraints = {
        intoAmount: new anchor.BN(1),
        intoPubkey: swapRecipientPubkey,
        intoAsset: asset
      }
      let blindingSwapInput = poseidonHash_1.poseidonHash(
        [
          swapConstraints.intoAmount,
          swapConstraints.intoPubkey,
          swapConstraints.intoAsset,
        ]
      );

      let swapInput = new light.Utxo(
        asset1,
        new anchor.BN(0),
        swapSetupKeypair,
        new anchor.BN(1),
        blindingSwapInput
      );

    }
    //
    else if (action == "SWAP_EXECUTE") {
      // mock keyPair which is used to
      let tmpKeypair = new keypair_1.Keypair();
      tmpKeypair.pubKey = swapConstraints.intoPubkey
      let blindingSwapInput = poseidonHash_1.poseidonHash(
        [
          swapConstraints.intoAmount,
          swapConstraints.intoPubkey,
          swapConstraints.intoAsset,
        ]
      );
      let deposit_utxoSwap = new light.Utxo(
        swapConstraints.intoAsset,
        new anchor.BN(swapConstraints.intoAmount),
        tmpKeypair,
        new anchor.BN(1), // instructionType
        poseidonHash_1.poseidonHash([ // blinding
            swapInput.blinding,
            swapInput.blinding
        ]),
      )
    }
    */
    console.log("inputUtxos.length ", inputUtxos.length);
    /// fill inputUtxos until 2 or 16
    while (inputUtxos.length !== 2 && inputUtxos.length < 10) {
      inputUtxos.push(new utxos_1.default(poseidon));
      // throw "inputUtxos.length > 2 are not implemented";
    }

    /// if there are no outputUtxo add one
    while (outputUtxos.length < 2) {
      outputUtxos.push(new utxos_1.default(poseidon));
    }
    /// mixes the input utxos
    /// mixes the output utxos
    shuffle = false
    if (shuffle) {
      console.log("shuffling utxos")

      inputUtxos = (0, shuffle_1.shuffle)(inputUtxos);
      outputUtxos = (0, shuffle_1.shuffle)(outputUtxos);

    } else {
      console.log("commented shuffle")
    }
    console.log("inputUtxos", inputUtxos[0]);
    console.log("outputUtxos", outputUtxos);

    /// the fee plus the amount to pay has to be bigger than the amount in the input utxo
    // which doesn't make sense it should be the other way arround right
    // the external amount can only be made up of utxos of asset[0]
    console.log("here before extNumber");
    const externalAmountBigNumber =  new anchor.BN(0)
        .add(outputUtxos.filter((utxo) => {return utxo.assets[1] == assets[1]}).reduce((sum, utxo) => (
          // add all utxos of the same asset
          // console.log("utxo add: ",  utxo.amount[1]);
          // console.log("utxo add asset: ",  utxo.assets[1]);
          // console.log("assets[1]: ", assets[1])
          // console.log("utxo add: ",  utxo.assets.toString()== assets[1].toString());
          sum.add(utxo.amounts[1])
          // if (utxo.assets.toString() == assets[1].toString()) {
          //   console.log("sum: ",  sum);
          //   sum.add(utxo.amount)
          //   console.log("sum: ",  sum);
          // }
        ), new anchor.BN(0)))
        .sub(inputUtxos.filter((utxo) => {return utxo.assets[1] == assets[1]}).reduce((sum, utxo) =>
          // console.log("utxo sub: ",  utxo.amount);
          //
          // if (utxo.assets == assets[1]) {
          //   sum.add(utxo.amount)
          // }
          sum.add(utxo.amounts[1]),
          new anchor.BN(0)
      ));
      console.log("here after extNumber");
    var feeAmount =  new anchor.BN(0)
        .add(outputUtxos.filter((utxo) => {return utxo.assets[0] == assets[0]}).reduce((sum, utxo) => (
          // add all utxos of the same asset
          // console.log("utxo add: ",  utxo.amount[1]);
          // console.log("utxo add asset: ",  utxo.assets[1]);
          // console.log("assets[1]: ", assets[1])
          // console.log("utxo add: ",  utxo.assets.toString()== assets[1].toString());
          sum.add(utxo.amounts[0])
          // if (utxo.assets.toString() == assets[1].toString()) {
          //   console.log("sum: ",  sum);
          //   sum.add(utxo.amount)
          //   console.log("sum: ",  sum);
          // }
        ), new anchor.BN(0)))
        .sub(inputUtxos.filter((utxo) => {return utxo.assets[0] == assets[0]}).reduce((sum, utxo) =>
          // console.log("utxo sub: ",  utxo.amount);
          //
          // if (utxo.assets == assets[1]) {
          //   sum.add(utxo.amount)
          // }
          sum.add(utxo.amounts[0]),
          new anchor.BN(0)
      ));

    /// if it is a deposit and the amount going in is smaller than 0 throw error
    if (enums_1.Action[action] === 'deposit' &&
        Number(externalAmountBigNumber.toString()) < 0) {
        throw new Error(`Incorrect Extamount: ${Number(externalAmountBigNumber.toString())}`);
    }

    outputUtxos.map((utxo) => {
      if (utxo.assets == null) {
        throw new Error(`output utxo asset not defined ${utxo}`);
      }
    });
    inputUtxos.map((utxo) => {
      if (utxo.assets == null) {
        throw new Error(`intput utxo asset not defined ${utxo}`);
      }
    });
    let assetPubkeys = [feeAsset,assets].concat();
    if (assets.length != 3) {
      throw new Error(`assetPubkeys.length != 3 ${assets}`);
    }
    // if (inputUtxos[0].asset != assets[0]) {
    //   throw new Error(`No feeUtxo in first place ${inputUtxos[0].asset}`);
    // }
    // let swapSetupConstraints = {
    //   swapConstraints,
    //   swapSetupKeypair
    // };

    let inIndices = []
    let outIndices = []
    console.log("assets.length. ", assets.length)

    if (assets[0] === assets[1] || assets[1] === assets[2] || assets[0] === assets[2]) {
      throw new Error(`asset pubKeys need to be distinct ${assets}`);
    }
    inputUtxos.map((utxo) => {
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
    })

    outputUtxos.map((utxo) => {
      let tmpOutIndices = []

      for (var a = 0; a < 3; a++) {
        let tmpOutIndices1 = []
          for (var i = 0; i < utxo.assets.length; i++) {
            if (utxo.assets[i] === assets[a]) {
              tmpOutIndices1.push("1")
            } else {
              tmpOutIndices1.push("0")
            }
          }
          tmpOutIndices.push(tmpOutIndices1)
      }
      outIndices.push(tmpOutIndices)

    })



    console.log("inIndices: ", inIndices)
    console.log("outIndices: ", outIndices)

    return {
        inputUtxos,
        outputUtxos,
        externalAmountBigNumber,
        inIndices,
        outIndices,
        feeAmount
        // constraint,
        // inInstructionType,
        // outInstructionType,
        // swapSetupConstraints
    };
};


exports.prepareUtxos = prepareUtxos;
// prepare transaction is in proof.js
