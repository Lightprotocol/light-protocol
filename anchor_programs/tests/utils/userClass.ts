const nacl = require('tweetnacl');
export const createEncryptionKeypair = () => nacl.box.keyPair();
const light = require('../../light-protocol-sdk');

export class UserClass {
  encryptionKeypair: any;
  shieldedKeypair: any;
  userUtxos: any;
  signature: any;
  constructor({ signature}) {
    this.signature = signature;
  }
  async initUser() {
    this.encryptionKeypair = deriveEncryptionKeypair(this.signature);
    this.shieldedKeypair = deriveShieldedKeypair(this.signature);
    this.userUtxos = fetchUserUtxos(this.shieldedKeypair);
  }
}

// Pseudo code  
function deriveEncryptionKeypair(signature) {
  return createEncryptionKeypair();
}

function deriveShieldedKeypair(signature) {
  return new light.Keypair();
}

function fetchUserUtxos(shieldedKeypair) {
  return '';
}
