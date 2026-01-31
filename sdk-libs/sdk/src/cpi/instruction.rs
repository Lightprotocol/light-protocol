use crate::{account::LightAccount, AnchorDeserialize, AnchorSerialize, LightDiscriminator, ProgramError};

#[cfg(feature = "poseidon")]
use crate::DataHasher;

/// Extension trait adding `with_light_account` to CPI instruction builders.
///
/// This is SDK-specific because it depends on [`LightAccount<A>`](crate::account::LightAccount),
/// which requires SHA256/Poseidon hashing. The base [`LightCpiInstruction`](light_sdk_interface::cpi::LightCpiInstruction)
/// trait from `light-sdk-interface` is framework-agnostic.
///
/// # Usage
///
/// Import this trait alongside [`LightCpiInstruction`](light_sdk_interface::cpi::LightCpiInstruction):
/// ```rust,ignore
/// use light_sdk::cpi::{LightCpiInstruction, WithLightAccount};
/// ```
pub trait WithLightAccount: Sized {
    /// Adds a compressed account to the instruction (using SHA256 hashing).
    ///
    /// The account can be an input (for updating/closing), output (for creating/updating),
    /// or both. The method automatically handles the conversion based on the account state.
    ///
    /// # Arguments
    /// * `account` - The light account to add to the instruction
    ///
    /// # Type Parameters
    /// * `A` - The compressed account data type
    #[must_use = "with_light_account returns a new value"]
    fn with_light_account<A>(self, account: LightAccount<A>) -> Result<Self, ProgramError>
    where
        A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + Default;

    /// Adds a compressed account to the instruction (using Poseidon hashing).
    ///
    /// Similar to [`with_light_account`](Self::with_light_account), but uses Poseidon hashing
    /// instead of SHA256. Use this when your compressed account data implements [`DataHasher`].
    ///
    /// # Arguments
    /// * `account` - The light account to add to the instruction
    ///
    /// # Type Parameters
    /// * `A` - The compressed account data type that implements DataHasher
    #[cfg(feature = "poseidon")]
    #[must_use = "with_light_account_poseidon returns a new value"]
    fn with_light_account_poseidon<A>(
        self,
        account: crate::account::poseidon::LightAccount<A>,
    ) -> Result<Self, ProgramError>
    where
        A: AnchorSerialize + AnchorDeserialize + LightDiscriminator + DataHasher + Default;
}
