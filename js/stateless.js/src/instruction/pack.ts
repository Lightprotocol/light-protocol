#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct SerializedUtxos {
    pub pubkey_array: Vec<Pubkey>,
    pub u64_array: Vec<u64>,
    pub in_utxos: Vec<InUtxoSerializable>,
    pub out_utxos: Vec<OutUtxoSerializable>,
}

impl SerializedUtxos {
   

    pub fn add_out_utxos(&mut self, utxos_to_add: &[OutUtxo], accounts: &[Pubkey]) -> Result<()> {
        for utxo in utxos_to_add.iter() {
            // Determine the owner index
            let owner_index = match accounts.iter().position(|&p| p == utxo.owner) {
                Some(index) => index as u8, // Found in accounts
                None => match self.pubkey_array.iter().position(|&p| p == utxo.owner) {
                    Some(index) => (accounts.len() + index) as u8, // Found in accounts
                    None => {
                        // Not found, add to pubkey_array and use index
                        self.pubkey_array.push(utxo.owner);
                        (accounts.len() + self.pubkey_array.len() - 1) as u8
                    }
                },
            };

            // Add the lamports index
            let lamports_index = match self.u64_array.iter().position(|&p| p == utxo.lamports) {
                Some(index) => index as u8, // Found in accounts
                None => {
                    // Not found, add to u64_array and use index
                    self.u64_array.push(utxo.lamports);
                    (self.u64_array.len() - 1) as u8
                }
            };

            // Serialize the UTXO data, if present
            let data_serializable = utxo.data.as_ref().map(|data| {
                // This transformation needs to be defined based on how Tlv can be converted to TlvSerializable
                Tlv::to_serializable_tlv(data, &mut self.pubkey_array, accounts)
            });

            // Create and add the InUtxoSerializable
            let in_utxo_serializable = OutUtxoSerializable {
                owner: owner_index,
                lamports: lamports_index,
                data: data_serializable,
            };
            self.out_utxos.push(in_utxo_serializable);
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
#[account]
pub struct InUtxoSerializable {
    pub owner: u8,
    pub leaf_index: u32,
    pub lamports: u8,
    pub data: Option<TlvSerializable>,
}

// no need to send blinding is computed onchain
#[derive(Debug, PartialEq)]
#[account]
pub struct OutUtxoSerializable {
    pub owner: u8,
    pub lamports: u8,
    pub data: Option<TlvSerializable>,
}

#[derive(Debug)]
#[account]
pub struct OutUtxo {
    pub owner: Pubkey,
    pub lamports: u64,
    pub data: Option<Tlv>,
}

// blinding we just need to send the leafIndex
#[derive(Debug, PartialEq)]
#[account]
pub struct Utxo {
    pub owner: Pubkey,
    pub blinding: [u8; 32],
    pub lamports: u64,
    pub data: Option<Tlv>,
}

impl Utxo {
    pub fn update_blinding(&mut self, merkle_tree_pda: Pubkey, index_of_leaf: usize) -> Result<()> {
        self.blinding = Poseidon::hashv(&[
            &hash(merkle_tree_pda.to_bytes().as_slice()).to_bytes()[0..30],
            index_of_leaf.to_le_bytes().as_slice(),
        ])
        .unwrap();
        Ok(())
    }
}



/// Time lock escrow example:
/// escrow tlv data -> compressed token program
/// let escrow_data = {
///   owner: Pubkey, // owner is the user pubkey
///   release_slot: u64,
///   deposit_slot: u64,
/// };
///
/// let escrow_tlv_data = TlvDataElement {
///   discriminator: [1,0,0,0,0,0,0,0],
///   owner: escrow_program_id,
///   data: escrow_data.try_to_vec()?,
/// };
/// let token_tlv = TlvDataElement {
///   discriminator: [2,0,0,0,0,0,0,0],
///   owner: token_program,
///   data: token_data.try_to_vec()?,
/// };
/// let token_data = Account {
///  mint,
///  owner,
///  amount: 10_000_000u64,
///  delegate: None,
///  state: Initialized, (u64)
///  is_native: None,
///  delegated_amount: 0u64,
///  close_authority: None,
/// };
///
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize, PartialEq)]
pub struct TlvDataElement {
    pub discriminator: [u8; 8],
    pub owner: Pubkey,
    pub data: Vec<u8>,
    pub data_hash: [u8; 32],
}