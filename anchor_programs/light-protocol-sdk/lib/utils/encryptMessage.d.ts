import nacl = require('tweetnacl');
export declare const encryptMessage: (message: any, nonce: any, recipientEncryptionKeypair: nacl.BoxKeyPair, senderThrowAwayKeypair: any) => Uint8Array;
