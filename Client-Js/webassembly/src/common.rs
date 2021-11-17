use ark_crypto_primitives::crh::constraints::{CRHGadget, TwoToOneCRHGadget};
use ark_crypto_primitives::crh::injective_map::constraints::{
    PedersenCRHCompressorGadget, TECompressorGadget,
};
use ark_crypto_primitives::crh::{
    injective_map::{PedersenCRHCompressor, TECompressor},
    pedersen,
};
use ark_ed_on_bls12_381::{constraints::EdwardsVar, EdwardsProjective};
use ark_sponge::CryptographicSponge;
use ark_sponge::poseidon::PoseidonParameters;
use ark_sponge::poseidon::PoseidonSponge;
use ark_sponge::constraints::AbsorbGadget;

pub type TwoToOneHash = PedersenCRHCompressor<EdwardsProjective, TECompressor, TwoToOneWindow>;
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TwoToOneWindow;

// `WINDOW_SIZE * NUM_WINDOWS` = 2 * 256 bits = enough for hashing two outputs.
impl pedersen::Window for TwoToOneWindow {
    const WINDOW_SIZE: usize = 4;
    const NUM_WINDOWS: usize = 128;
}

pub type LeafHash = PedersenCRHCompressor<EdwardsProjective, TECompressor, LeafWindow>;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LeafWindow;

// `WINDOW_SIZE * NUM_WINDOWS` = 2 * 256 bits = enough for hashing two outputs.
impl pedersen::Window for LeafWindow {
    const WINDOW_SIZE: usize = 4;
    const NUM_WINDOWS: usize = 144;
}

pub type NullifierHash = PedersenCRHCompressor<EdwardsProjective, TECompressor, NullifierWindow>;


#[derive(Clone, PartialEq, Eq, Hash)]
pub struct NullifierWindow;

// `WINDOW_SIZE * NUM_WINDOWS` = 2 * 256 bits = enough for hashing two outputs.
impl pedersen::Window for NullifierWindow {
    const WINDOW_SIZE: usize = 2;
    const NUM_WINDOWS: usize = 144;
}


pub type TwoToOneHashGadget = PedersenCRHCompressorGadget<
    EdwardsProjective,
    TECompressor,
    TwoToOneWindow,
    EdwardsVar,
    TECompressorGadget,
>;

pub type LeafHashGadget = PedersenCRHCompressorGadget<
    EdwardsProjective,
    TECompressor,
    LeafWindow,
    EdwardsVar,
    TECompressorGadget,
>;

pub type NullifierHashGadget = PedersenCRHCompressorGadget<
    EdwardsProjective,
    TECompressor,
    NullifierWindow,
    EdwardsVar,
    TECompressorGadget,
>;
/*
pub type PoseidonHashGadget = PedersenCRHCompressorGadget<
    EdwardsProjective,
    TECompressor,
    NullifierWindow,
    EdwardsVar,
    TECompressorGadget,
>;
*/
pub type LeafHashParamsVar = <LeafHashGadget as CRHGadget<LeafHash, ConstraintF>>::ParametersVar;
pub type TwoToOneHashParamsVar =
    <TwoToOneHashGadget as TwoToOneCRHGadget<TwoToOneHash, ConstraintF>>::ParametersVar;

pub type NullifierParamsVar = <NullifierHashGadget as CRHGadget<NullifierHash, ConstraintF>>::ParametersVar;

pub type Parameters_p = PoseidonParameters::<ConstraintF>;

//pub type PoseidonParamsVar = <ark_sponge::poseidon::constraints::PoseidonSpongeVar::<ConstraintF> as ark_sponge::constraints::CryptographicSpongeVar<ConstraintF, CryptographicSponge<Parameters = Parameters_p>>>::Parameters;
//<PoseidonSpongeVar<F> as CryptographicSpongeVar<F, PoseidonSponge<F>>>
//pub type PoseidonParamsVar = <ark_sponge::poseidon::constraints::PoseidonSpongeVar::<ConstraintF> as ark_sponge::constraints::CryptographicSpongeVar<ConstraintF, PoseidonSponge::<ConstraintF>>>::Parameters;
//pub type PoseidonParamsVar = <ark_sponge::poseidon::constraints::PoseidonSpongeVar::<ConstraintF> as AbsorbGadget<ConstraintF>>::PoseidonSpongeVar;
//pub type PoseidonSpongeVar = <ark_sponge::poseidon::constraints::PoseidonSpongeVar::<ConstraintF> as ark_sponge::constraints::CryptographicSpongeVar<ConstraintF, PoseidonSponge::<ConstraintF>>>::Parameters;

pub type ConstraintF = ark_ed_on_bls12_381::Fq;
