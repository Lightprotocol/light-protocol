# Mock App Verifier

This repo serves as a minimal testcase to test verifier_program_two which can only be invoked via cpi.

Additionally, this repo can be used as a template to build a private Solana program.

A private Solana program consists out of a circuit which encodes the application logic and a verifier Solana program.
This program


- Private application logic
- shielded assets
- fees can be paid in shielded assets to a relayer



To interact with the verifier program the light-sdk Transaction class takes care of the inputs.


## Prerequisites:
- Circom // TODO: add links
- Anchor // TODO: add links
- Light Protocol Architecture // TODO: add links


# Building a Private Solana Program (PSP)


### You need to implement:
- application logic in circuit (/circuit/app_transaction.circom)
- implement verifier class (/sdk/src/verifier.ts)
- adapt appParams in functional test (tests/functional_test.ts)
- add checks for public inputs in the verifier program (programs/verifer/src/lib.rs & processor.rs)

### Application Logic

A circom circuit encodes the logic of the application. The computation of the logic is private. As a developer you
can decide what information to expose, by specifying public inputs in your circom contract.
Apart from transparency public inputs are data you need to send to the blockchain which is a limited resource.
Public inputs can also be useful to check data in the verifier program at runtime. An example is a timelock escrow utxo.
In this case the user creates a utxo which is encodes the hash of the release slot, as the instruction type hash and the verifier program publickey in the verifier address field.

You have access to the utxos, and all data encoded in those, which are inputs to the system proof.
For every utxo:
- amount sol
- amount spl asset
- spl asset pubkey (hashed and truncated to fit 254bit)
- blinding
- shielded public key
- instruction hash
- pool type
- verifier publickey


**Example timelock escrow:**

IMPORTANT: this circuit is only verified when the an app utxo of this circuit is spent. When the app utxo is inserted no checks other than normal transfer checks are necessary. It might make sense to make utxo insert transactions look the same as utxo spent transactions for additional privacy. For the utxo insertion if the app utxo data fits into the storage of system verifier zero or one (174bytes and 256 bytes) these verifiers can be used to deposit app utxos.

Add releaseSlot as circuit input, define which input utxo is the 

In ./circuit/app_transaction.circom
```
    // just wrote this never compiled it
    signal input releaseSlot;
    signal input isEscrowAccount[nIns];

    // calculate instruction hash
    component instructionHash = Poseidon(1);
    instructionHash.in[0] <== releaseSlot;


    component checkInstructionType[nIns];
    // search for input utxo with this instructionHash
    // the position of this utxo is specified in isEscrowAccount
    // This is necessary because all paths need inside the circuit are hardcoded.
    for (var i=0; i < nIns; i++) {          
        checkInstructionType[i][j] = ForceEqualIfEnabled();
        checkInstructionType[i][j].in[0] <== inInstructionType[i];
        checkInstructionType[i][j].in[1] <== instructionHash.out;
        checkInstructionType[i][j].enabled <== isEscrowAccount[i];
    }
```


Declare releaseSlot a public input in ./circuit/appTransaction.circom
```
component main {public [connectingHash, verifier, releaseSlot]} = TransactionMarketPlace(18, 4, 4, 24603683191960664281975569809895794547840992286820815015841170051925534051, 0, 1, 3, 2, 2, 1);

```

### App Verifier Program

The app verifier program inherits the verifying functionality from the verifier sdk. 
Additional public inputs can be specified in checked public inputs.

Furthermore, the developer is free to enforce checks over public inputs, and execute any additional logic.

Example check to lock funds of an utxo until a Solana slot.
```
use anchor_lang::solana_program::sysvar;

let current_slot = <Clock as sysvar::Sysvar>::get()?.slot;

if current_slot
    < u64::from_be_bytes(
        ctx.accounts.verifier_state.checked_public_inputs[2][24..32]
            .try_into()
            .unwrap(),
    )
{
    return err!(crate::AppVerifierError::FundsAreLocked);
}

```

A the spending of a light app utxo needs two separate Solana transactions, because the amount of data is too large to fit into a single transaction. The first transaction sends a bulk of data. This data is stored in a temporary account which is created in the first transaction and closed in the second transaction. This temporary account is also used to pass data to the system verifier via cpi.



### Verifier.ts Class
Naming of these methods must not change since the Transaction object expects these methods to exist.

The verifier config has to be the same as the app enabled system verifier the app verifier program calls.
At this point in time there is only one app enabled verifier, verifier two.
```
config = { in: 4, out: 4, app: true };
```

**Implement methods:**
- parsePublicInputsFromArray()
    - creates an object from the public inputs array returned from proof generation in the Transaction class
    - if you added additional public inputs in the circuit you need to add those here in same the order as they are declared in the circom file
- getInstructions():
    - use the anchor library and IDL to generate the instructions
    - in case of extra inputs these need to be added according to your app verifier program implementation

### functional test

Add your inputs to the appParams object.

```
const appParams = {
    verifier: new MockVerifier(),
    inputs: { 
        releaseSlot,
    },
}
```


### Client Class (advanced)

// TODO: add a template from the verifier market place but not sure when and to what degree this makes sense

For complicated circuits you might want to implement a class to generate the appParams object.