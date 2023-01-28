


### UTXO App Extension

Idea: just append utxo data to be encrypted with the extra data.
To know how to read the data we check a pda with derivation path verifierPubkey, ".

**Serialized Utxo**
[blinding[31], amountFee[8], amountAsset[8], assetId[8], message[32], verifierPubkey[32], appData[~]]
nonce[24] 
padding[16]
length(noAppData): 127
length(appData): 127 + 32 (verifierPubkey) + appData.length

Options:
var buf = Buffer.from(JSON.stringify(obj));
var temp = JSON.parse(buf.toString());

{
    price: u64,
    authPubKey: Pubkey,
    authEncPubKey: [u8; 32k],
    utxoPrivKey: [u8; 32],
    symKeyEnc: [u8;32 * encParties]
}



### Offer

1. make
Action: put nft in escrow
InUtxos:
- nft
- (feeUtxo)
outUtxos:
- escrowUtxo
- changeUtxo
DATA: utxo plus Shielded Privatekey, change utxo, empty data

2. take
Action: execute escrow utxo
ReqUtxos: escrow utxo, paying utxo, 
InUtxos:
- escrow
- paying
- (paying)
- (paying)
outUtxos:
- escrowUtxo
- changeUtxo
- marketPlaceFee
- Royalty
DATA: normal

3. cancel
Action: invalidate escrow utxo
InUtxos:
- escrow
- feeUtxo
outUtxos:
- nft
- changeUtxo
DATA: normal

4. fetch

### Bid
bid is a reverse offer, the bidder offers x sol in exchange for one nft.

1. make
Action: put bid amount in escrow
InUtxos:
- bidAsset
- (feeUtxo)
- (bidAsset)
- (bidAsset)
outUtxos:
- escrowUtxo
- changeUtxo
DATA: utxo plus Shielded Privatekey, change utxo, empty data

2. take
Action: execute escrow utxo
InUtxos:
- escrow
- paying
- (paying)
- (paying)
outUtxos:
- escrowUtxo
- changeUtxo
- marketPlaceFee
- Royalty
DATA: normal

3. cancel
Action: invalidate escrow utxo
InUtxos:
- escrow
- feeUtxo
outUtxos:
- bidAsset
- changeUtxo
DATA: normal

4. fetch

Wallet:

keep funds balanced between 2 utxos -> can always withdraw all funds at once, can do two smaller tx in parallel


