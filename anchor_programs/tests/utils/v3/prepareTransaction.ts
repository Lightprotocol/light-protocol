const enums_1 = require("./enums");
const utxos_1 = require("./utxos").default;
const anchor = require("@project-serum/anchor")
const constants_1 = require("./constants");
const shuffle_1 = require("./shuffle");

const feeAsset = new anchor.BN(anchor.web3.SystemProgram.programId._bn.toString()).mod(constants_1.FIELD_SIZE)
// prepare transaction is in proof.js
export const prepareUtxos = (
    inputUtxos = [],
    outputUtxos = [],
    assets = [],
    action,
    poseidon,
    shuffle = true,
  ) => {
    /// Validation
    if (inputUtxos.length > 16 || outputUtxos.length > 4) {
        throw new Error('Incorrect inputUtxos/outputUtxos count');
    }
 
    /// fill inputUtxos until 2 or 16
    while (inputUtxos.length !== 2 && inputUtxos.length < 16) {
      inputUtxos.push(new utxos_1.default(poseidon));
      // throw "inputUtxos.length > 2 are not implemented";
    }

    /// if there are no outputUtxo add one
    while (outputUtxos.length < 2) {
      outputUtxos.push(new utxos_1.default(poseidon));
    }
    /// mixes the input utxos
    /// mixes the output utxos
    if (shuffle) {
      console.log("shuffling utxos")

      inputUtxos = (0, shuffle_1.shuffle)(inputUtxos);
      outputUtxos = (0, shuffle_1.shuffle)(outputUtxos);

    } else {
      console.log("commented shuffle")
    }
    /// the fee plus the amount to pay has to be bigger than the amount in the input utxo
    // which doesn't make sense it should be the other way arround right
    // the external amount can only be made up of utxos of asset[0]
    console.log("here before extNumber");
    const externalAmountBigNumber =  new anchor.BN(0)
        .add(outputUtxos.filter((utxo) => {return utxo.assets[1] == assets[1]}).reduce((sum, utxo) => (
          sum.add(utxo.amounts[1])
        ), new anchor.BN(0)))
        .sub(inputUtxos.filter((utxo) => {return utxo.assets[1] == assets[1]}).reduce((sum, utxo) =>
          sum.add(utxo.amounts[1]),
          new anchor.BN(0)
      ));
      console.log("here after extNumber");
    var feeAmount =  new anchor.BN(0)
        .add(outputUtxos.filter((utxo) => {return utxo.assets[0] == assets[0]}).reduce((sum, utxo) => (
          sum.add(utxo.amounts[0])
        ), new anchor.BN(0)))
        .sub(inputUtxos.filter((utxo) => {return utxo.assets[0] == assets[0]}).reduce((sum, utxo) =>
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
      throw new Error(`assetPubkeys.length != 3 ${assetPubkeys}`);
    }

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
    };
};


