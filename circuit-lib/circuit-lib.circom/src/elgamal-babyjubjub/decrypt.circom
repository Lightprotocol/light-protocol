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

}


// TODO: double check the precompute
// We are first going to prove the correct precompute
// Then we are going to prove the correct decode using the precompute
template Decode (nBits) {
    var base[2] = [
        5299619240641551281634865583518297030282874472190772894086521144482721001553,
        16950150798460657717958625567821834550301663161624707787222815936182638968203
    ];
    var precomputeSize = 19;
    var precomputeUpperBound = 2**precomputeSize;
    signal input xhi;
    signal input xlo;
    signal input encodedMessage[2];
    signal output decodedMessage;
    var scalar = xhi * precomputeUpperBound;

    component scalarBits = Num2Bits(253);
    scalarBits.in <== scalar;
    // the output yields the key for the precompute
    component baseMultiplyPrecompute = EscalarMulFix(253, base);
    for  (var i=0; i<253; i++) {
        baseMultiplyPrecompute.e[i] <==scalarBits.out[i];
    }
    component scalarXloBits = Num2Bits(253);
    scalarXloBits.in <== xlo;
    // the output yields the key for the precompute
    component baseMultiplyXlo = EscalarMulFix(253, base);
    for  (var i=0; i<253; i++) {
        baseMultiplyXlo.e[i] <== scalarXloBits.out[i];
    }
    component negatedEncodedMessage = NegateBabyJubJubPoint();
    negatedEncodedMessage.x <== encodedMessage[0];
    negatedEncodedMessage.y <== encodedMessage[1];

    // component checkNegatedEncodedMessage = BabyCheck();
    // checkNegatedEncodedMessage.x <== negatedEncodedMessage.negatedX;
    // checkNegatedEncodedMessage.y <== negatedEncodedMessage.negatedY;

    component sub = BabyAdd();
    sub.x1 <== negatedEncodedMessage.negatedX;
    sub.x2 <== baseMultiplyXlo.out[0];
    sub.y1 <==  negatedEncodedMessage.negatedY;
    sub.y2 <== baseMultiplyXlo.out[1];
    // log(sub.xout);
    // log(baseMultiplyPrecompute.out[0]);
    // log(sub.yout);
    // log(baseMultiplyPrecompute.out[1]);
    // TODO: check why it breaks sometimes
    // baseMultiplyPrecompute.out[0] === sub.xout;
    // baseMultiplyPrecompute.out[1] === sub.yout;
    var range = 32 - precomputeSize;
    var rangeBound = 2**range;
    decodedMessage <== xlo + xhi * rangeBound;
}

template NegateBabyJubJubPoint() {
    signal input x;
    signal input y;
    signal output negatedX;
    signal output negatedY;

    negatedX <== -x;  // Negating the x-coordinate
    negatedY <== y;   // Keeping the y-coordinate the same
}