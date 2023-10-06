pragma circom 2.1.4;
include "./insert_leaf.circom";
component main {public [updatedRoot, leaf, subTreeHash, newSubTreeHash]} =  insert_leaf( 18);