const crypto = require("crypto");
const { ethers } = require("ethers");
const BigNumber = ethers.BigNumber;
const { poseidon } = require("circomlib");
var ffjavascript = require("ffjavascript");
const { unstringifyBigInts, leInt2Buff, beInt2Buff } = ffjavascript.utils;

// const poseidonHash = (items) => BigNumber.from(poseidon(items).toString());
function poseidonHash(items) {
    return BigNumber.from(poseidon(items).toString());
}
// const poseidonHash2 = (a, b) => poseidonHash([a, b]);

const FIELD_SIZE = BigNumber.from(
  "21888242871839275222246405745257275088548364400416034343698204186575808495617",
);
// const privateKeyDecoded =
/** Generate random number of specified byte length */
const randomBN = (nbytes = 31) => BigNumber.from(crypto.randomBytes(nbytes));

// function getExtDataHash({
//   recipient,
//   extAmount,
//   relayer,
//   fee,
//   encryptedOutput1,
//   encryptedOutput2,
// }) {
//   const abi = new ethers.utils.AbiCoder();

//   const encodedData = abi.encode(
//     [
//       "tuple(address recipient,int256 extAmount,address relayer,uint256 fee,bytes encryptedOutput1,bytes encryptedOutput2)",
//     ],
//     [
//       {
//         recipient: toFixedHex(recipient, 20),
//         extAmount: toFixedHex(extAmount),
//         relayer: toFixedHex(relayer, 20),
//         fee: toFixedHex(fee),
//         encryptedOutput1: encryptedOutput1,
//         encryptedOutput2: encryptedOutput2,
//       },
//     ],
//   );
//   const hash = ethers.utils.keccak256(encodedData);
//   return BigNumber.from(hash).mod(FIELD_SIZE);
// }

function getExtDataHash({
  // inputs are bytes
  recipient,
  extAmount,
  relayer,
  fee,
  merkleTreePubkeyBytes,
  encryptedOutput1,
  nonce1,
  senderThrowAwayPubkey1,
  encryptedOutput2,
  nonce2,
  senderThrowAwayPubkey2,
}) {
  console.log("...recipient", recipient);
  console.log("...extAmount", extAmount);
  console.log("...relayer", relayer);
  console.log("...fee", fee);
  console.log("...mt pubkey ", merkleTreePubkeyBytes);
  console.log("...encryptedOutput1", encryptedOutput1);
  console.log("...nonce1", nonce1);
  console.log("...senderThrowAwayPubkey1", senderThrowAwayPubkey1);

  console.log("...encryptedOutput2", encryptedOutput2);

  let encodedData = new Uint8Array([
    ...recipient, // [0..32]
    ...extAmount, // 8
    ...relayer,
    ...fee, // 32
    ...merkleTreePubkeyBytes, // 32
    0, // index of merkletreetokenpda : part []
    ...encryptedOutput1, // 216
    ...nonce1,
    ...senderThrowAwayPubkey1,
    ...encryptedOutput2,
    ...nonce2,
    ...senderThrowAwayPubkey2,
    // ...[0],
  ]);
  console.log("encodedData:", encodedData);
  console.log("nonce 1", nonce1);
  console.log("encr output1", encryptedOutput1);
  console.log("senderThrowAwayPubkey1", senderThrowAwayPubkey1);

  const hash = ethers.utils.keccak256(Buffer.from(encodedData));

  return {
    extDataHash: BigNumber.from(hash).mod(FIELD_SIZE),
    extDataBytes: encodedData,
  };
}

//
/** BigNumber to hex string of specified length */
function toFixedHex(number, length = 32) {
  let result =
    "0x" +
    (number instanceof Buffer
      ? number.toString("hex")
      : BigNumber.from(number).toHexString().replace("0x", "")
    ).padStart(length * 2, "0");
  if (result.indexOf("-") > -1) {
    result = "-" + result.replace("-", "");
  }
  return result;
}

/** Convert value into buffer of specified byte length */
const toBuffer = (value, length) =>
  Buffer.from(
    BigNumber.from(value)
      .toHexString()
      .slice(2)
      .padStart(length * 2, "0"),
    "hex",
  );

const toPad = (value, length) =>
  Buffer.from(
    BigNumber.from(value)
      .toHexString()
      .slice(2)
      .padStart(length * 2, "0"),
    "hex",
  );
const intToBuffer = (hash, len = 32) =>
  beInt2Buff(unstringifyBigInts(hash), len);

const testest = (data, bytes = 32) =>
  leInt2Buff(unstringifyBigInts(data), bytes);

const leInt2Buffer = (data, bytes = 32) =>
  leInt2Buff(unstringifyBigInts(data), bytes);

function shuffle(array) {
  let currentIndex = array.length;
  let randomIndex;

  // While there remain elements to shuffle...
  while (0 !== currentIndex) {
    // Pick a remaining element...
    randomIndex = Math.floor(Math.random() * currentIndex);
    currentIndex--;

    // And swap it with the current element.
    [array[currentIndex], array[randomIndex]] = [
      array[randomIndex],
      array[currentIndex],
    ];
  }

  return array;
}

// async function getSignerFromAddress(address) {
//   await network.provider.request({
//     method: "hardhat_impersonateAccount",
//     params: [address],
//   });

//   return await ethers.provider.getSigner(address);
// }

async function parseInputsToBytes(data) {
  var mydata = JSON.parse(data.toString());

  for (var i in mydata) {
    mydata[i] = leInt2Buff(unstringifyBigInts(mydata[i]), 32).toString();
  }

  // fs.writeFile("publicInputsBytes.txt", JSON.stringify(mydata), function (err) {
  //   if (err) {
  //     return console.error(err);
  //   }
  // });
  return mydata;
}

async function parseInputsToBytesArray(data) {
  var mydata = JSON.parse(data.toString());

  for (var i in mydata) {
    mydata[i] = leInt2Buff(unstringifyBigInts(mydata[i]), 32);
  }

  // fs.writeFile("publicInputsBytes.txt", mydata, function (err) {
  //   if (err) {
  //     return console.error(err);
  //   }
  // });
  // if toString() cant do that //l:125
  let x = [];
  mydata.map((array) => {
    array.map((byte) => {
      x.push(byte);
    });
  });
  return x;
}

async function parseProofToBytes(data) {
  var mydata = JSON.parse(data.toString());

  for (var i in mydata) {
    if (i == "pi_a") {
      for (var j in mydata[i]) {
        mydata[i][j] = leInt2Buff(
          unstringifyBigInts(mydata[i][j]),
          32, // 48
        ).toString();
      }
    } else if (i == "pi_b") {
      for (var j in mydata[i]) {
        for (var z in mydata[i][j]) {
          mydata[i][j][z] = leInt2Buff(
            unstringifyBigInts(mydata[i][j][z]),
            32, // 48
          ).toString();
        }
      }
    } else if (i == "pi_c") {
      for (var j in mydata[i]) {
        mydata[i][j] = leInt2Buff(
          unstringifyBigInts(mydata[i][j]),
          32, //48
        ).toString();
      }
    }
  }
  let mydataStripped = {
    pi_a: mydata.pi_a,
    pi_b: mydata.pi_b,
    pi_c: mydata.pi_c,
  };
  // fs.writeFile(
  //   "proofBytes.txt",
  //   JSON.stringify(mydataStripped),
  //   function (err) {
  //     if (err) {
  //       return console.error(err);
  //     }
  //   }
  // );
  return mydataStripped;
}

async function parseProofToBytesArray(data) {
  var mydata = JSON.parse(data.toString());

  for (var i in mydata) {
    if (i == "pi_a") {
      for (var j in mydata[i]) {
        mydata[i][j] = leInt2Buff(
          unstringifyBigInts(mydata[i][j]),
          32, // 48
        );
      }
    } else if (i == "pi_b") {
      for (var j in mydata[i]) {
        for (var z in mydata[i][j]) {
          mydata[i][j][z] = leInt2Buff(
            unstringifyBigInts(mydata[i][j][z]),
            32, // 48
          );
        }
      }
    } else if (i == "pi_c") {
      for (var j in mydata[i]) {
        mydata[i][j] = leInt2Buff(
          unstringifyBigInts(mydata[i][j]),
          32, //48
        );
      }
    }
  }
  let mydataStripped = [
    mydata.pi_a[0],
    mydata.pi_a[1],
    mydata.pi_b[0],
    mydata.pi_b[1],
    mydata.pi_c[0],
    mydata.pi_c[1],
  ];
  var merged = [].concat.apply([], mydataStripped);
  let x = [];
  merged.map((array) => {
    array.map((byte) => {
      x.push(byte);
    });
  });

  // fs.writeFile("proofBytesArray.txt", JSON.stringify(x), function (err) {
  //   if (err) {
  //     return console.error(err);
  //   }
  // });
  return x;
}

module.exports = {
  FIELD_SIZE,
  randomBN,
  toFixedHex,
  toBuffer,
  intToBuffer,
  poseidonHash,
  // poseidonHash2,
  getExtDataHash,
  shuffle,
  // getSignerFromAddress,
  parseInputsToBytes,
  parseProofToBytes,
  parseProofToBytesArray,
  parseInputsToBytesArray,
  testest,
};

// export {
//   FIELD_SIZE,
//   randomBN,
//   toFixedHex,
//   toBuffer,
//   intToBuffer,
//   poseidonHash,
//   poseidonHash2,
//   getExtDataHash,
//   shuffle,
//   // privateKeyDecoded,
//   // getSignerFromAddress,
//   parseInputsToBytes,
//   parseProofToBytes,
//   parseProofToBytesArray,
//   parseInputsToBytesArray,
//   testest,
//   leInt2Buffer,
// };
