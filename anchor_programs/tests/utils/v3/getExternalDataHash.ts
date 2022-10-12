const ethers_1 = require('ethers');
const anchor = require('@project-serum/anchor');
const toBufferLE = require('bigint-buffer');

const constants_1 = require('../constants');
export const getExtDataHash = function (
  // inputs are bytes
  recipient,
  recipient_fee,
  relayer,
  relayer_fee,
  merkleTreeIndex,
  encryptedUtxos
) {
  console.log('recipient ', Array.from(recipient));
  console.log('recipient_fee ', Array.from(recipient_fee));
  console.log('relayer ', Array.from(relayer));
  console.log('relayer_fee ', relayer_fee);
  console.log('index merkletreetokenpda ', merkleTreeIndex);
  console.log('encryptedUtxos ', encryptedUtxos.toString());

  let encodedData = new Uint8Array([
    ...recipient,
    ...recipient_fee,
    ...relayer,
    ...relayer_fee,
    merkleTreeIndex,
    ...encryptedUtxos,
    // ...[0],
  ]);
  const hash = ethers_1.ethers.utils.keccak256(Buffer.from(encodedData));
  console.log('hash_ethers ', hash);
  console.log(
    'hash_ethers mod',
    ethers_1.BigNumber.from(hash).mod(constants_1.FIELD_SIZE_ETHERS)
  );
  console.log(
    'hash: ',
    Array.from(
      anchor.utils.bytes.hex.decode(
        ethers_1.BigNumber.from(hash).mod(constants_1.FIELD_SIZE_ETHERS)._hex
      )
    )
  );
  return {
    extDataHash: ethers_1.BigNumber.from(hash).mod(
      constants_1.FIELD_SIZE_ETHERS
    ), //new anchor.BN(anchor.utils.bytes.hex.decode(hash)).mod(constants_1.FIELD_SIZE),
    extDataBytes: encodedData,
  };
};
