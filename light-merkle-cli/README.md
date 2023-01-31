# Merkle Tree Config CLI

This CLI is a command line interface for managing and configuring a Light Protocol Merkle Tree on Solana blockchain. It allows you to perform various operations such as adding new authorities, initializing a Merkle Tree, registering verifiers, adding to a pool, coriguring the merkle authority and logging the tree.

## Installation

To use this CLI, you will need to have Node.js and npm installed on your system. You can then install the package by running the following command:

```shell
npm install -g merkle-tree-config-cli
```

**Note:** package is not released yet you need to follow the below process to run the cli locally

**Step 1:** Clone the light-protocol-onchain repository and solana repository in a single directory

```shell

mkdir light-protocol 

cd light-protocol

git clone https://github.com/Lightprotocol/light-protocol-onchain

git clone https://github.com/ananas-block/solana/tree/master

cd solana 

git checkout 656b150e575a4d16cfa9c9ff63b16edcf94f2e0d

```

**Note:** Make sure you switch to dev_v4-mt-cli branch in light-protocol-onchain 

**Step 2:** run the local test validator from light-protocol-onchain

```shell

../solana/validator/solana-test-validator \
--reset \
--limit-ledger-size 500000000 \
--bpf-program J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i ./light-system-programs/target/deploy/verifier_program_zero.so \
--bpf-program JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6 ./light-system-programs/target/deploy/merkle_tree_program.so \
--bpf-program 3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL ./light-system-programs/target/deploy/verifier_program_one.so


```

**Step 3:** Build and setup

```shell

cd light-protocol-onchain/light-merkle-cli

npm install

npm run build:sdk

npm run start:dev

npm run setup

```

**The CLI is now ready for use**

## Usage

To get a list of all available commands and options in the CLI, run the following command:

```shell

merkle-tree-config-cli help

```

### Creating an authority account

An authority account is an account that is authorized to perform certain operations on the Merkle Tree.

```shell
# create a new authority account

merkle-tree-config-cli authority init

```

```shell

# get the authority account information 
merkle-tree-config-cli authority get -p <Pubkey>
# here PubKey is the address of the authority account
merkle-tree-config-cli authority get 5EMc8sCbHeb1HtRFifcbCiXN66kX6Wddrd61EkdJun6Y

```

```shell

# update the authority in the authority account 
merkle-tree-config-cli authority set -p <Pubkey>
# here PubKey is the pubKey of the new authority 
merkle-tree-config-cli authority set 8aiVquZc9ijcmDMYzhTS6b3j3SWfzMkPdkpg4Ux2bQBx

```

**Note:** If you are changing the authority from one to another make sure you have right payer key in the id.json file , you can found it in the directory


### Initializing a Merkle Tree

This will create a new Merkle Tree on the Solana blockchain. To initialize a Merkle Tree, run the following command:

```shell

merkle-tree-config-cli initialize -p <PubKey>
# Here pubkey is the address where merkle tree account is created
merkle-tree-config-cli initialize DCxUdYgqjE6AR9m13VvqpkxJqGJYnk8jn8NEeD3QY3BU

```


### Registering a verifier

A verifier is a public key that is authorized to verify proofs of inclusion in the Merkle Tree. To register a verifier, run the following command:

```shell 

# register a new verfier for the merkle tree
merkle-tree-config-cli verifier set -p <publicKey>
# Where <publicKey> is the public key of the verifier you want to register.

```

```shell 
# get a verifier details 
merkle-tree-config-cli verifier get -p <publicKey>
# Where <publicKey> is the public key of the verifier you want to register.

```

```shell 
merkle-tree-config-cli verifier set J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i
merkle-tree-config-cli verifier set 3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL
merkle-tree-config-cli verifier get 3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL
merkle-tree-config-cli verifier set GFDwN8PXuKZG2d2JLxRhbggXYe9eQHoGYoYK5K3G5tV8
merkle-tree-config-cli verifier get GFDwN8PXuKZG2d2JLxRhbggXYe9eQHoGYoYK5K3G5tV8

```

### Adding to a pool
 
To Register a pool, run the following command:

```shell 

merkle-tree-config-cli pool pooltype
# register the pool for the merkle-tree

```
```shell 

merkle-tree-config-cli pool sol
# register the sol pool for the merkle-tree

```

### Configuring the Merkle Tree

The Merkle Tree has several configurable parameters, such as enabling the nfts , permissionless tokens or updating the lockDuration of the merkle tree. To configure the Merkle Tree, run the following command:

```shell

 # enable the nft in authority account
 merkle-tree-config-cli configure nfts
 merkle-tree-config-cli authority get 5EMc8sCbHeb1HtRFifcbCiXN66kX6Wddrd61EkdJun6Y

```

```shell

 # enable the permissionless spl token
 merkle-tree-config-cli configure spl
 merkle-tree-config-cli authority get 5EMc8sCbHeb1HtRFifcbCiXN66kX6Wddrd61EkdJun6Y

```

```shell
 
 # update the lockDuration in the merkleTree
 merkle-tree-config-cli configure lock 2000

```

### Printing the Merkle Tree
To print the Merkle Tree, run the following command:

```shell

merkle-tree-config-cli print <Pubkey>

# Here pubKey is the address of the merkle tree
# This will output the current state of the Merkle Tree, including the root hash and the number of leaves.

merkle-tree-config-cli print 5EMc8sCbHeb1HtRFifcbCiXN66kX6Wddrd61EkdJun6Y

```

### List All Accounts
To list all accounts ( merkleTree, authority, verifier, pool and tokens), run the following command:

```shell

merkle-tree-config-cli list

````
