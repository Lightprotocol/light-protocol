use borsh::{BorshDeserialize, BorshSerialize};
use light_compressed_account::Pubkey;

use super::compressed_mint::BaseMint;

// Manual implementation of BorshSerialize for SPL compatibility
impl BorshSerialize for BaseMint {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // Write mint_authority as COption (4 bytes + 32 bytes)
        if let Some(authority) = self.mint_authority {
            writer.write_all(&[1, 0, 0, 0])?; // COption Some discriminator
            writer.write_all(&authority.to_bytes())?;
        } else {
            writer.write_all(&[0; 36])?; // COption None (4 bytes) + empty pubkey (32 bytes)
        }

        // Write supply (8 bytes)
        writer.write_all(&self.supply.to_le_bytes())?;

        // Write decimals (1 byte)
        writer.write_all(&[self.decimals])?;

        // Write is_initialized (1 byte)
        writer.write_all(&[if self.is_initialized { 1 } else { 0 }])?;

        // Write freeze_authority as COption (4 bytes + 32 bytes)
        if let Some(authority) = self.freeze_authority {
            writer.write_all(&[1, 0, 0, 0])?; // COption Some discriminator
            writer.write_all(&authority.to_bytes())?;
        } else {
            writer.write_all(&[0; 36])?; // COption None (4 bytes) + empty pubkey (32 bytes)
        }

        Ok(())
    }
}

// Manual implementation of BorshDeserialize for SPL compatibility
impl BorshDeserialize for BaseMint {
    fn deserialize_reader<R: std::io::Read>(buf: &mut R) -> std::io::Result<Self> {
        // Read mint_authority COption
        let mut discriminator = [0u8; 4];
        buf.read_exact(&mut discriminator)?;
        let mut pubkey_bytes = [0u8; 32];
        buf.read_exact(&mut pubkey_bytes)?;
        let mint_authority = if u32::from_le_bytes(discriminator) == 1 {
            Some(Pubkey::from(pubkey_bytes))
        } else {
            None
        };

        // Read supply
        let mut supply_bytes = [0u8; 8];
        buf.read_exact(&mut supply_bytes)?;
        let supply = u64::from_le_bytes(supply_bytes);

        // Read decimals
        let mut decimals = [0u8; 1];
        buf.read_exact(&mut decimals)?;
        let decimals = decimals[0];

        // Read is_initialized
        let mut is_initialized = [0u8; 1];
        buf.read_exact(&mut is_initialized)?;
        let is_initialized = is_initialized[0] != 0;

        // Read freeze_authority COption
        let mut discriminator = [0u8; 4];
        buf.read_exact(&mut discriminator)?;
        let mut pubkey_bytes = [0u8; 32];
        buf.read_exact(&mut pubkey_bytes)?;
        let freeze_authority = if u32::from_le_bytes(discriminator) == 1 {
            Some(Pubkey::from(pubkey_bytes))
        } else {
            None
        };

        Ok(Self {
            mint_authority,
            supply,
            decimals,
            is_initialized,
            freeze_authority,
        })
    }
}
