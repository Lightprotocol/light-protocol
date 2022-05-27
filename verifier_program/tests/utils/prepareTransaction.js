require("dotenv").config();
const PROGRAM_ID = process.env.REACT_APP_PROGRAM_ID;
const REACT_APP_MERKLE_TREE_PDA_PUBKEY =
  process.env.REACT_APP_MERKLE_TREE_PDA_PUBKEY;

const { U64, I64 } = require("n64");
const nacl = require("tweetnacl");
const MerkleTree = require("./merkleTree").default;
const Utxo = require("./utxo").default;
const { prove } = require("./prover");
const {
  toFixedHex,
  poseidonHash2,
  getExtDataHash,
  FIELD_SIZE,
  shuffle,
  parseProofToBytesArray,
  parseInputsToBytesArray,
} = require("./utils");

const solana = require("@solana/web3.js");
const token = require("@solana/spl-token");

const { ethers } = require("ethers");
const { BigNumber } = ethers;

const MERKLE_TREE_HEIGHT = 18;
const newNonce = () => nacl.randomBytes(nacl.box.nonceLength);
const newKeypair = () => nacl.box.keyPair();

async function buildMerkleTree({ connection }) {
  const program_pubKey = new solana.PublicKey(PROGRAM_ID);
  var leave_accounts = await connection.getProgramAccounts(program_pubKey, {
    filters: [{ dataSize: 106 + 222 }],
  });

  var leaves_to_sort = [];
  leave_accounts.map((acc) => {
    leaves_to_sort.push({
      index: U64(acc.account.data.slice(2, 10)).toString(),
      leaves: acc.account.data.slice(10, 74),
    });
  });
  leaves_to_sort.sort((a, b) => parseFloat(a.index) - parseFloat(b.index));

  var leaves = [];
  for (var i = 0; i < leave_accounts.length; i++) {
    leaves.push(toFixedHex(leaves_to_sort[i].leaves.slice(0, 32).reverse()));
    leaves.push(toFixedHex(leaves_to_sort[i].leaves.slice(32, 64).reverse()));
  }

  console.log("leaves fetched: ", leaves);
  return new MerkleTree(MERKLE_TREE_HEIGHT, leaves, {
    hashFunction: poseidonHash2,
  });
}

async function getProof({
  inputs,
  outputs,
  tree,
  extAmountBn,
  fee,
  recipient,
  relayer,
  action,
  recipientEncryptionPubkey,
  recipientEncryptionKeypairTEST,
  shieldedKeypairTEST,
}) {
  inputs = shuffle(inputs);
  outputs = shuffle(outputs);

  let inputMerklePathIndices = [];
  let inputMerklePathElements = [];

  for (const input of inputs) {
    if (input.amount > 0) {
      input.index = tree.indexOf(toFixedHex(input.getCommitment()));
      if (input.index < 0) {
        throw new Error(
          `Input commitment ${toFixedHex(input.getCommitment())} was not found`,
        );
      }
      inputMerklePathIndices.push(input.index);
      inputMerklePathElements.push(tree.path(input.index).pathElements);
    } else {
      inputMerklePathIndices.push(0);
      inputMerklePathElements.push(new Array(tree.levels).fill(0));
    }
  }

  let int64;
  if (extAmountBn._hex.indexOf("-") > -1) {
    // is withdrawal
    int64 = I64(-1 * Number(extAmountBn._hex.slice(1)));
  } else {
    int64 = I64(Number(extAmountBn._hex));
  }
  let z = new Uint8Array(8);
  int64.writeLE(z, 0);

  let feesLE = new Uint8Array(8);
  if (action != "deposit") {
    fee.writeLE(feesLE, 0);
  } else {
    feesLE.fill(0);
  }

  // Encrypting outputs only
  const nonces = [newNonce(), newNonce()];
  const senderThrowAwayKeypairs = [newKeypair(), newKeypair()];
  /// Encrypt outputs to bytes
  let encryptedOutputs = [];

  outputs.map((x, index) =>
    encryptedOutputs.push(
      x.encrypt(
        nonces[index],
        recipientEncryptionPubkey,
        senderThrowAwayKeypairs[index],
      ),
    ),
  );

  // test decrypt: same?

  console.log("encr outputs: ", encryptedOutputs);
  console.log("encr outputs lengths: ", encryptedOutputs[0].length);
  console.log("encr outputs lengths: ", encryptedOutputs[1].length);

  const extData = {
    recipient: new solana.PublicKey(recipient).toBytes(),
    extAmount: z,
    relayer: new solana.PublicKey(relayer).toBytes(),
    fee: feesLE,
    merkleTreePubkeyBytes: new solana.PublicKey(
      REACT_APP_MERKLE_TREE_PDA_PUBKEY,
    ).toBytes(),
    encryptedOutput1: encryptedOutputs[0],
    encryptedOutput2: encryptedOutputs[1],
    nonce1: nonces[0],
    nonce2: nonces[1],
    senderThrowAwayPubkey1: senderThrowAwayKeypairs[0].publicKey,
    senderThrowAwayPubkey2: senderThrowAwayKeypairs[1].publicKey,
  };

  let { extDataHash, extDataBytes } = getExtDataHash(extData);

  let input = {
    root: tree.root(),
    inputNullifier: inputs.map((x) => x.getNullifier()),
    outputCommitment: outputs.map((x) => x.getCommitment()),
    publicAmount: BigNumber.from(extAmountBn)
      .sub(BigNumber.from(fee.toString()))
      .add(FIELD_SIZE)
      .mod(FIELD_SIZE)
      .toString(),
    extDataHash,

    // data for 2 transaction inputs
    inAmount: inputs.map((x) => x.amount),
    inPrivateKey: inputs.map((x) => x.keypair.privkey),
    inBlinding: inputs.map((x) => x.blinding),
    inPathIndices: inputMerklePathIndices,
    inPathElements: inputMerklePathElements,

    // data for 2 transaction outputs
    outAmount: outputs.map((x) => x.amount),
    outBlinding: outputs.map((x) => x.blinding),
    outPubkey: outputs.map((x) => x.keypair.pubkey),
  };

  const { proofJson, publicInputsJson } = await prove(
    input,
    `./artifacts/circuits/transaction${inputs.length}`, // gui
    // `./public/artifacts/circuits/transaction${inputs.length}`, // cli
  );

  const args = {
    action,
    recipient,
    relayer,
    outputs,
  };

  return {
    data: {
      extAmount: extData.extAmount,
      extAmountBn,
      extDataBytes,
      publicInputsBytes: await parseInputsToBytesArray(publicInputsJson),
      proofBytes: await parseProofToBytesArray(proofJson),
      encryptedOutputs: encryptedOutputs, // may need these somewhere
    },
    args,
  };
}

async function prepareTransaction({
  inputs = [],
  outputs = [],
  fee = 0,
  recipient = 0,
  relayer = 0,
  connection,
  action,
  recipientEncryptionPubkey,
  recipientEncryptionKeypairTEST,
  shieldedKeypairTEST,
}) {
  console.log(
    "performing prepareTransaction, inputs, outputs",
    inputs,
    outputs,
  );
  if (inputs.length > 16 || outputs.length > 2) {
    throw new Error("Incorrect inputs/outputs count");
  }
  while (inputs.length !== 2 && inputs.length < 16) {
    inputs.push(new Utxo());
  }
  while (outputs.length < 2) {
    outputs.push(new Utxo());
  }

  console.log("after:, inputs, outputs", inputs, outputs);
  let rent = await token.Token.getMinBalanceRentForExemptAccount(connection);
  console.log("DEV: tokenRent for rentexempt: ", rent);

  let extAmountBn = BigNumber.from(fee.toString())
    .add(outputs.reduce((sum, x) => sum.add(x.amount), BigNumber.from(0)))
    .sub(inputs.reduce((sum, x) => sum.add(x.amount), BigNumber.from(0)))
    //.sub(BigNumber.from(rent.toString()));

  //fee = fee;
  console.log("DEV: EXTaMOUNTBN: ", Number(extAmountBn));
  const { args, data } = await getProof({
    inputs,
    outputs,
    tree: await buildMerkleTree({ connection }),
    extAmountBn,
    fee,
    recipient,
    relayer,
    action,
    recipientEncryptionPubkey,
    recipientEncryptionKeypairTEST,
    shieldedKeypairTEST,
  });

  return {
    args,
    data,
  };
}

module.exports = prepareTransaction ;
