import nacl from 'tweetnacl';

let circomlibjs = require('circomlibjs');
const light = require('../../light-protocol-sdk');
export const createEncryptionKeypair = () => nacl.box.keyPair();

export class UserClass {
  encryptionKeypair: nacl.BoxKeyPair;
  shieldedKeypair: any;
  userUtxos: any;
  signature: string;
  constructor({ signature}) {
    this.signature = signature;
  }
  async initUser(POSEIDON) {
    const encryptionKeypair = deriveEncryptionKeypair(this.signature);
    const shieldedKeypair = await deriveShieldedKeypair(this.signature, POSEIDON);
    const userUtxos = fetchUserUtxos(shieldedKeypair);
    return {encryptionKeypair, shieldedKeypair, userUtxos, POSEIDON};
  }
}

// Pseudo code  
function deriveEncryptionKeypair(signature) {
  return createEncryptionKeypair();
}

async function deriveShieldedKeypair(signature, POSEIDON) {
  // const POSEIDON = await circomlibjs.buildPoseidonOpt();
  const shieldedKeypair = new light.Keypair(POSEIDON)
  console.log("Shielded Keypair in userClass", shieldedKeypair)
  return shieldedKeypair;
}

function fetchUserUtxos(shieldedKeypair) {
  return '';
}
