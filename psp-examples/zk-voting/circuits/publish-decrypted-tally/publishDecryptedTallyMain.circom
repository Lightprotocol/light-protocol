pragma circom 2.1.4;
include "./publish_decrypted_tally.circom";

component main {public [
    publicVoteWeightYesX,
    publicVoteWeightYesY,
    publicVoteWeightYesEmphemeralKeyX,
    publicVoteWeightYesEmphemeralKeyY,
    publicVoteWeightNoX,
    publicVoteWeightNoY,
    publicVoteWeightNoEmphemeralKeyX,
    publicVoteWeightNoEmphemeralKeyY,
    publicYesResult,
    publicNoResult
]} =  publish_decrypted_tally();