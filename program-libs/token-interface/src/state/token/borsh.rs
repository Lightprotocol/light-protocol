use crate::{AnchorDeserialize, AnchorSerialize};
use light_compressed_account::Pubkey;

use crate::state::{AccountState, ExtensionStruct, Token, ACCOUNT_TYPE_TOKEN_ACCOUNT};

// Manual implementation of BorshSerialize for SPL compatibility
impl AnchorSerialize for Token {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // Write mint (32 bytes)
        writer.write_all(&self.mint.to_bytes())?;

        // Write owner (32 bytes)
        writer.write_all(&self.owner.to_bytes())?;

        // Write amount (8 bytes)
        writer.write_all(&self.amount.to_le_bytes())?;

        // Write delegate as COption (4 bytes + 32 bytes)
        if let Some(delegate) = self.delegate {
            writer.write_all(&[1, 0, 0, 0])?; // COption Some discriminator
            writer.write_all(&delegate.to_bytes())?;
        } else {
            writer.write_all(&[0; 36])?; // COption None (4 bytes) + empty pubkey (32 bytes)
        }

        // Write state (1 byte)
        writer.write_all(&[self.state as u8])?;

        // Write is_native as COption (4 bytes + 8 bytes)
        if let Some(is_native) = self.is_native {
            writer.write_all(&[1, 0, 0, 0])?; // COption Some discriminator
            writer.write_all(&is_native.to_le_bytes())?;
        } else {
            writer.write_all(&[0; 12])?; // COption None (4 bytes) + empty u64 (8 bytes)
        }

        // Write delegated_amount (8 bytes)
        writer.write_all(&self.delegated_amount.to_le_bytes())?;

        // Write close_authority as COption (4 bytes + 32 bytes)
        if let Some(close_authority) = self.close_authority {
            writer.write_all(&[1, 0, 0, 0])?; // COption Some discriminator
            writer.write_all(&close_authority.to_bytes())?;
        } else {
            writer.write_all(&[0; 36])?; // COption None (4 bytes) + empty pubkey (32 bytes)
        }

        // End of SPL Token Account base layout (165 bytes)

        // Write account_type and extensions only if extensions are present
        if self.extensions.is_some() {
            // Write account_type byte at position 165
            writer.write_all(&[self.account_type])?;

            // Write extensions as Option<Vec<ExtensionStruct>>
            self.extensions.serialize(writer)?;
        }

        Ok(())
    }
}

// Manual implementation of BorshDeserialize for SPL compatibility
impl AnchorDeserialize for Token {
    fn deserialize_reader<R: std::io::Read>(buf: &mut R) -> std::io::Result<Self> {
        // Read mint (32 bytes)
        let mut mint_bytes = [0u8; 32];
        buf.read_exact(&mut mint_bytes)?;
        let mint = Pubkey::from(mint_bytes);

        // Read owner (32 bytes)
        let mut owner_bytes = [0u8; 32];
        buf.read_exact(&mut owner_bytes)?;
        let owner = Pubkey::from(owner_bytes);

        // Read amount (8 bytes)
        let mut amount_bytes = [0u8; 8];
        buf.read_exact(&mut amount_bytes)?;
        let amount = u64::from_le_bytes(amount_bytes);

        // Read delegate COption (4 bytes + 32 bytes)
        let mut discriminator = [0u8; 4];
        buf.read_exact(&mut discriminator)?;
        let mut pubkey_bytes = [0u8; 32];
        buf.read_exact(&mut pubkey_bytes)?;
        let delegate = if u32::from_le_bytes(discriminator) == 1 {
            Some(Pubkey::from(pubkey_bytes))
        } else {
            None
        };

        // Read state (1 byte)
        let mut state = [0u8; 1];
        buf.read_exact(&mut state)?;
        let state = state[0];

        // Read is_native COption (4 bytes + 8 bytes)
        let mut discriminator = [0u8; 4];
        buf.read_exact(&mut discriminator)?;
        let mut value_bytes = [0u8; 8];
        buf.read_exact(&mut value_bytes)?;
        let is_native = if u32::from_le_bytes(discriminator) == 1 {
            Some(u64::from_le_bytes(value_bytes))
        } else {
            None
        };

        // Read delegated_amount (8 bytes)
        let mut delegated_amount_bytes = [0u8; 8];
        buf.read_exact(&mut delegated_amount_bytes)?;
        let delegated_amount = u64::from_le_bytes(delegated_amount_bytes);

        // Read close_authority COption (4 bytes + 32 bytes)
        let mut discriminator = [0u8; 4];
        buf.read_exact(&mut discriminator)?;
        let mut pubkey_bytes = [0u8; 32];
        buf.read_exact(&mut pubkey_bytes)?;
        let close_authority = if u32::from_le_bytes(discriminator) == 1 {
            Some(Pubkey::from(pubkey_bytes))
        } else {
            None
        };

        // End of SPL Token Account base layout (165 bytes)

        // Try to read account_type byte at position 165
        let mut account_type_byte = [0u8; 1];
        let (account_type, extensions) = if buf.read_exact(&mut account_type_byte).is_ok() {
            let account_type = account_type_byte[0];
            if account_type == ACCOUNT_TYPE_TOKEN_ACCOUNT {
                // Read extensions
                let extensions =
                    Option::<Vec<ExtensionStruct>>::deserialize_reader(buf).unwrap_or_default();
                (account_type, extensions)
            } else {
                // Account type byte present but not Token - store it but no extensions
                (account_type, None)
            }
        } else {
            // No account_type byte - base SPL token account without extensions
            // Default to ACCOUNT_TYPE_TOKEN_ACCOUNT for Token
            (ACCOUNT_TYPE_TOKEN_ACCOUNT, None)
        };

        Ok(Self {
            mint,
            owner,
            amount,
            delegate,
            state: AccountState::try_from(state)
                .map_err(|e| std::io::Error::from_raw_os_error(u32::from(e) as i32))?,
            is_native,
            delegated_amount,
            close_authority,
            account_type,
            extensions,
        })
    }
}
