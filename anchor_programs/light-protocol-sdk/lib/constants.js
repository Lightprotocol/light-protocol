"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.DEFAULT_ZERO = exports.FIELD_SIZE = exports.PROGRAM_ID = exports.RELAYER_ADDRESS = exports.LIGHTSHIELD_WIDGET = exports.MERKLE_TREE_HEIGHT = exports.REACT_APP_MERKLE_TREE_PDA_PUBKEY = exports.REACT_APP_RELAYER_URL = exports.RPC_URL = exports.NEXT_PUBLIC_PROGRAM_ID = void 0;
const ethers_1 = require("ethers");
const anchor = require("@project-serum/anchor")

const solana = require('@solana/web3.js');
exports.NEXT_PUBLIC_PROGRAM_ID = '2c54pLrGpQdGxJWUAoME6CReBrtDbsx5Tqx4nLZZo6av';
const REACT_APP_RELAYER_PUBKEY = 'CZBQHCfGQwUqMQTCk7oeMBtsKcm4KS5g8KuaAwmNriie';
const REACT_APP_PROGRAM_ID = 'JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6';
exports.RPC_URL = 'https://solana-api.syndica.io/access-token/tANLeSckea5t1rS6YnKbfgrNgWA5gUdt45e9KcffFSO8hFmHriW9JsGjTNVHHl75/rpc';
// export const REACT_APP_RELAYER_URL = "https://light-relayer-v1-mainnet-vkgo7.ondigitalocean.app"
// export const REACT_APP_RELAYER_URL = 'https://light-relayer-fsenl.ondigitalocean.app'
exports.REACT_APP_RELAYER_URL = 'https://relay.lightprotocol.com';
// TODO WE WORK ON LOCALHOST RIGHT NOW
// console.log('WE WORK ON LOCALHOST RIGHT NOW!!!!!!!!')
//export const REACT_APP_RELAYER_URL = 'http://localhost:3001'
exports.REACT_APP_MERKLE_TREE_PDA_PUBKEY = 'DpdgARh2mTCgCkCawPzQYG5q9miYB8H5cFQWYXASJSy9';
exports.MERKLE_TREE_HEIGHT = 18;
exports.LIGHTSHIELD_WIDGET = 'https://widget.lightprotocol.com'; //'https://widget.lightprotocol.com'
// TODO How is this an address if it is made from a function called publicKey
exports.RELAYER_ADDRESS = new solana.PublicKey(REACT_APP_RELAYER_PUBKEY);
exports.PROGRAM_ID = new solana.PublicKey(REACT_APP_PROGRAM_ID);
// is this const?
exports.FIELD_SIZE = new anchor.BN('21888242871839275222246405745257275088548364400416034343698204186575808495617');
exports.DEFAULT_ZERO = '14522046728041339886521211779101644712859239303505368468566383402165481390632';
