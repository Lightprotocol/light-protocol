#!/bin/bash -e
solana airdrop 100000 ZBUKxVWviAJBy12edp5H6kvhcatGYW3BV4ijbgxpVSq && solana airdrop 100000 ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k && solana airdrop 100000 8Ers2bBEWExdrh7KDFTrRbauPbFeEvsHz3UX4vxcK9xY && solana airdrop 10000 BEKmoiPHRUxUPik2WQuKqkoFLLkieyNPrTDup5h8c9S7
cd ../relayer && node lib/index.js
PID=$!
$1;
kill $PID;