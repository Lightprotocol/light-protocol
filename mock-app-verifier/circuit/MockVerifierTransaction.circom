pragma circom 2.0.0;
include "./mock_verifier_transaction.circom";
component main {public [currentSlot, transactionHash, publicAppVerifier]} =  mockVerifierTransaction( 18, 4, 4, 24603683191960664281975569809895794547840992286820815015841170051925534051, 0, 1, 3, 2, 2);