pragma circom 2.1.2;

include "babyjub.circom";
include "bitify.circom";
include "escalarmulany.circom";

template Decrypt() {

    // sender's encrypted message
    signal input ciphertext[2];   
    // sender's ephemeral key              
    signal input ephemeralKey[2];  
    // receiver's private key               
    signal input secretKey;                     

    // decrypted message => [secretKey].ephemeralKey - decryptedMessage
    signal output decryptedMessage[2];                
    
    // assert the encrypted message is a point on curve
    component encryptedMessageIsOnCurve = BabyCheck();
    encryptedMessageIsOnCurve.x <== ciphertext[0];
    encryptedMessageIsOnCurve.y <== ciphertext[1];

    // assert the ephemeral key is a point on curve
    component ephemeralKeyIsOnCurve = BabyCheck();         
    ephemeralKeyIsOnCurve.x <== ephemeralKey[0];
    ephemeralKeyIsOnCurve.y <== ephemeralKey[1];
    
    
    component escalarMul = EscalarMulAny(253);
    escalarMul.p[0] <== ephemeralKey[0];
    escalarMul.p[1] <== ephemeralKey[1];

    var i;
    component secretKeyBits = Num2Bits(253);
    secretKey ==> secretKeyBits.in;
    for  (i=0; i<253; i++) {
        secretKeyBits.out[i] ==> escalarMul.e[i];
    }

    signal inversedMaskingKey[2];
    inversedMaskingKey[0] <== - escalarMul.out[0];
    inversedMaskingKey[1] <== escalarMul.out[1];
    
    component add = BabyAdd();
    
    add.x1 <== ciphertext[0];
    add.y1 <== ciphertext[1];
    add.x2 <== inversedMaskingKey[0];
    add.y2 <== inversedMaskingKey[1];

    decryptedMessage[0] <== add.xout;
    decryptedMessage[1] <== add.yout;
}

