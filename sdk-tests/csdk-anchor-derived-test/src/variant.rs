use anchor_lang::prelude::*;
use light_ctoken_sdk::{
    compat::{CTokenData, PackedCTokenData},
    pack::Pack as TokenPack,
};
use light_sdk::{
    account::Size,
    compressible::{CompressionInfo, HasCompressionInfo, Pack as SdkPack, Unpack as SdkUnpack},
    instruction::{account_meta::CompressedAccountMetaNoLamportsNoAddress, PackedAccounts},
    LightDiscriminator,
};

use crate::{
    instruction_accounts::DecompressAccountsIdempotent,
    seeds::get_ctoken_signer_seeds,
    state::{
        GameSession, PackedGameSession, PackedPlaceholderRecord, PackedUserRecord,
        PlaceholderRecord, UserRecord,
    },
};

#[derive(AnchorSerialize, AnchorDeserialize, Debug, Clone, Copy)]
#[repr(u8)]
pub enum CTokenAccountVariant {
    CTokenSigner = 0,
}

impl light_ctoken_sdk::compressible::CTokenSeedProvider for CTokenAccountVariant {
    type Accounts<'info> = DecompressAccountsIdempotent<'info>;

    fn get_seeds<'a, 'info>(
        &self,
        accounts: &'a Self::Accounts<'info>,
        _remaining_accounts: &'a [AccountInfo<'info>],
    ) -> std::result::Result<(Vec<Vec<u8>>, Pubkey), ProgramError> {
        match self {
            CTokenAccountVariant::CTokenSigner => {
                // Use the same convention as the mint/init path: ("ctoken_signer", user, mint)
                std::result::Result::<(Vec<Vec<u8>>, Pubkey), ProgramError>::Ok(
                    get_ctoken_signer_seeds(&accounts.fee_payer.key(), &accounts.some_mint.key()),
                )
            }
        }
    }

    fn get_authority_seeds<'a, 'info>(
        &self,
        _accounts: &'a Self::Accounts<'info>,
        _remaining_accounts: &'a [AccountInfo<'info>],
    ) -> std::result::Result<(Vec<Vec<u8>>, Pubkey), ProgramError> {
        // Not used by the decompression runtime in this test.
        std::result::Result::<(Vec<Vec<u8>>, Pubkey), ProgramError>::Err(
            ProgramError::InvalidAccountData,
        )
    }
}

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
pub enum CompressedAccountVariant {
    UserRecord(UserRecord),
    PackedUserRecord(PackedUserRecord),
    GameSession(GameSession),
    PackedGameSession(PackedGameSession),
    PlaceholderRecord(PlaceholderRecord),
    PackedPlaceholderRecord(PackedPlaceholderRecord),
    PackedCTokenData(PackedCTokenData<CTokenAccountVariant>),
    CTokenData(CTokenData<CTokenAccountVariant>),
}

impl Default for CompressedAccountVariant {
    fn default() -> Self {
        Self::UserRecord(UserRecord::default())
    }
}

impl LightDiscriminator for CompressedAccountVariant {
    const LIGHT_DISCRIMINATOR: [u8; 8] = [0; 8];
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
}

impl HasCompressionInfo for CompressedAccountVariant {
    fn compression_info(&self) -> &CompressionInfo {
        match self {
            Self::UserRecord(data) => data.compression_info(),
            Self::GameSession(data) => data.compression_info(),
            Self::PlaceholderRecord(data) => data.compression_info(),
            _ => unreachable!(),
        }
    }

    fn compression_info_mut(&mut self) -> &mut CompressionInfo {
        match self {
            Self::UserRecord(data) => data.compression_info_mut(),
            Self::GameSession(data) => data.compression_info_mut(),
            Self::PlaceholderRecord(data) => data.compression_info_mut(),
            _ => unreachable!(),
        }
    }

    fn compression_info_mut_opt(&mut self) -> &mut Option<CompressionInfo> {
        match self {
            Self::UserRecord(data) => data.compression_info_mut_opt(),
            Self::GameSession(data) => data.compression_info_mut_opt(),
            Self::PlaceholderRecord(data) => data.compression_info_mut_opt(),
            _ => unreachable!(),
        }
    }

    fn set_compression_info_none(&mut self) {
        match self {
            Self::UserRecord(data) => data.set_compression_info_none(),
            Self::GameSession(data) => data.set_compression_info_none(),
            Self::PlaceholderRecord(data) => data.set_compression_info_none(),
            _ => unreachable!(),
        }
    }
}

impl Size for CompressedAccountVariant {
    fn size(&self) -> usize {
        match self {
            Self::UserRecord(data) => data.size(),
            Self::GameSession(data) => data.size(),
            Self::PlaceholderRecord(data) => data.size(),
            _ => unreachable!(),
        }
    }
}

impl SdkPack for CompressedAccountVariant {
    type Packed = Self;

    fn pack(&self, remaining_accounts: &mut PackedAccounts) -> Self::Packed {
        match self {
            Self::UserRecord(data) => Self::PackedUserRecord(data.pack(remaining_accounts)),
            Self::GameSession(data) => Self::PackedGameSession(data.pack(remaining_accounts)),
            Self::PlaceholderRecord(data) => {
                Self::PackedPlaceholderRecord(data.pack(remaining_accounts))
            }
            Self::CTokenData(data) => {
                Self::PackedCTokenData(TokenPack::pack(data, remaining_accounts))
            }
            _ => unreachable!(),
        }
    }
}

impl SdkUnpack for CompressedAccountVariant {
    type Unpacked = Self;

    fn unpack(
        &self,
        remaining_accounts: &[AccountInfo],
    ) -> std::result::Result<Self::Unpacked, ProgramError> {
        match self {
            Self::PackedUserRecord(data) => Ok(Self::UserRecord(data.unpack(remaining_accounts)?)),
            Self::PackedGameSession(data) => {
                Ok(Self::GameSession(data.unpack(remaining_accounts)?))
            }
            Self::PackedPlaceholderRecord(data) => {
                Ok(Self::PlaceholderRecord(data.unpack(remaining_accounts)?))
            }
            Self::PackedCTokenData(data) => Ok(Self::PackedCTokenData(data.clone())),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug, AnchorDeserialize, AnchorSerialize)]
pub struct CompressedAccountData {
    pub meta: CompressedAccountMetaNoLamportsNoAddress,
    pub data: CompressedAccountVariant,
}
