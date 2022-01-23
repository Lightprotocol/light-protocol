#[cfg(test)]
pub mod tests {

	use ark_serialize::Write;

	use ark_bn254;
	use ark_ed_on_bn254;

	use ark_std::vec::Vec;

	use ark_ff::biginteger::BigInteger256;
	use serde_json::{Result, Value};
	use std::fs;

	use ark_ff::bytes::FromBytes;
	use ark_ff::Fp256;
	use ark_ff::QuadExtField;
	use ark_groth16::prepare_verifying_key;
	use ark_groth16::{prepare_inputs, verify_proof};

	use ark_ec;
	use ark_ec::ProjectiveCurve;
	use ark_ff::Field;
	use light_protocol_core::groth16_verifier::final_exponentiation::{
		instructions::*, processor::_process_instruction, ranges::*, state::FinalExpBytes,
	};
	use light_protocol_core::groth16_verifier::parsers::*;
	use light_protocol_core::utils::prepared_verifying_key::*;
	use solana_program::program_pack::Pack;
	use std::fs::File;

	pub fn get_pvk_from_bytes_254() -> Result<
		ark_groth16::data_structures::VerifyingKey<ark_ec::models::bn::Bn<ark_bn254::Parameters>>,
	> {
		let contents = fs::read_to_string("./tests/test_data/verification_key_bytes_254.txt")
			.expect("Something went wrong reading the file");
		let v: Value = serde_json::from_str(&contents)?;

		let mut a_g1_bigints = Vec::new();
		for i in 0..2 {
			let mut bytes: Vec<u8> = Vec::new();
			for i in v["vk_alpha_1"][i].as_str().unwrap().split(',') {
				bytes.push((*i).parse::<u8>().unwrap());
			}
			a_g1_bigints
				.push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
		}
		let alpha_g1_bigints = ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
			a_g1_bigints[0],
			a_g1_bigints[1],
			false,
		);

		let mut b_g2_bigints = Vec::new();
		for i in 0..2 {
			for j in 0..2 {
				let mut bytes: Vec<u8> = Vec::new();
				for z in v["vk_beta_2"][i][j].as_str().unwrap().split(',') {
					bytes.push((*z).parse::<u8>().unwrap());
				}
				b_g2_bigints
					.push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
			}
		}

		let beta_g2 = ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(
			QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
				b_g2_bigints[0],
				b_g2_bigints[1],
			),
			QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
				b_g2_bigints[2],
				b_g2_bigints[3],
			),
			false,
		);

		let mut delta_g2_bytes = Vec::new();
		for i in 0..2 {
			for j in 0..2 {
				let mut bytes: Vec<u8> = Vec::new();
				for z in v["vk_delta_2"][i][j].as_str().unwrap().split(',') {
					bytes.push((*z).parse::<u8>().unwrap());
				}
				delta_g2_bytes
					.push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
			}
		}

		let delta_g2 = ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(
			QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
				delta_g2_bytes[0],
				delta_g2_bytes[1],
			),
			QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
				delta_g2_bytes[2],
				delta_g2_bytes[3],
			),
			false,
		);

		let mut gamma_g2_bytes = Vec::new();
		for i in 0..2 {
			for j in 0..2 {
				let mut bytes: Vec<u8> = Vec::new();
				for z in v["vk_gamma_2"][i][j].as_str().unwrap().split(',') {
					bytes.push((*z).parse::<u8>().unwrap());
				}
				gamma_g2_bytes
					.push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
			}
		}
		let gamma_g2 = ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(
			QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
				gamma_g2_bytes[0],
				gamma_g2_bytes[1],
			),
			QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
				gamma_g2_bytes[2],
				gamma_g2_bytes[3],
			),
			false,
		);

		let mut gamma_abc_g1_bigints_bytes = Vec::new();

		for i in 0..8 {
			let mut g1_bytes = Vec::new();
			for u in 0..2 {
				let mut bytes: Vec<u8> = Vec::new();
				for z in v["IC"][i][u].as_str().unwrap().split(',') {
					bytes.push((*z).parse::<u8>().unwrap());
				}
				g1_bytes
					.push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
			}
			gamma_abc_g1_bigints_bytes.push(ark_ec::models::bn::g1::G1Affine::<
				ark_bn254::Parameters,
			>::new(g1_bytes[0], g1_bytes[1], false));
		}

		Ok(ark_groth16::data_structures::VerifyingKey::<
			ark_ec::models::bn::Bn<ark_bn254::Parameters>,
		> {
			alpha_g1: alpha_g1_bigints,
			beta_g2: beta_g2,
			gamma_g2: gamma_g2,
			delta_g2: delta_g2,
			gamma_abc_g1: gamma_abc_g1_bigints_bytes,
		})
	}

	fn get_proof_from_bytes_254(
	) -> Result<ark_groth16::data_structures::Proof<ark_ec::models::bn::Bn<ark_bn254::Parameters>>>
	{
		let contents = fs::read_to_string("./tests/test_data/proof_bytes_254.txt")
			.expect("Something went wrong reading the file");
		let v: Value = serde_json::from_str(&contents)?;

		let mut a_g1_bigints = Vec::new();
		for i in 0..2 {
			let mut bytes: Vec<u8> = Vec::new();
			for i in v["pi_a"][i].as_str().unwrap().split(',') {
				bytes.push((*i).parse::<u8>().unwrap());
			}
			a_g1_bigints
				.push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
		}
		let a_g1 = ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
			a_g1_bigints[0],
			a_g1_bigints[1],
			false,
		);

		let mut b_g2_bigints = Vec::new();
		for i in 0..2 {
			for j in 0..2 {
				let mut bytes: Vec<u8> = Vec::new();
				for z in v["pi_b"][i][j].as_str().unwrap().split(',') {
					bytes.push((*z).parse::<u8>().unwrap());
				}
				b_g2_bigints
					.push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
			}
		}
		let b_g2 = ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(
			QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
				b_g2_bigints[0],
				b_g2_bigints[1],
			),
			QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
				b_g2_bigints[2],
				b_g2_bigints[3],
			),
			false,
		);

		let mut c_g1_bigints = Vec::new();
		for i in 0..2 {
			let mut bytes: Vec<u8> = Vec::new();
			for i in v["pi_c"][i].as_str().unwrap().split(',') {
				bytes.push((*i).parse::<u8>().unwrap());
			}
			c_g1_bigints
				.push(<Fp256<ark_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap());
		}
		let c_g1 = ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
			c_g1_bigints[0],
			c_g1_bigints[1],
			false,
		);

		Ok(
			ark_groth16::data_structures::Proof::<ark_ec::models::bn::Bn<ark_bn254::Parameters>> {
				a: a_g1,
				b: b_g2,
				c: c_g1,
			},
		)
	}

	pub fn get_public_inputs_from_bytes_254() -> Result<Vec<Fp256<ark_ed_on_bn254::FqParameters>>> {
		let contents = fs::read_to_string("./tests/test_data/public_inputs_254_bytes.txt")
			.expect("Something went wrong reading the file");
		let v: Value = serde_json::from_str(&contents)?;

		let mut res = Vec::new();
		for i in 0..7 {
			let mut bytes: Vec<u8> = Vec::new();
			for i in v[i].as_str().unwrap().split(',') {
				bytes.push((*i).parse::<u8>().unwrap());
			}
			res.push(<Fp256<ark_ed_on_bn254::FqParameters> as FromBytes>::read(&bytes[..]).unwrap())
		}

		Ok(res)
	}

	#[test]
	fn hardcoded_verifyingkey_test() {
		let pvk_unprepped = get_pvk_from_bytes_254().unwrap();
		let pvk = prepare_verifying_key(&pvk_unprepped);
		assert_eq!(get_gamma_abc_g1_0(), pvk.vk.gamma_abc_g1[0]);
		assert_eq!(get_gamma_abc_g1_1(), pvk.vk.gamma_abc_g1[1]);
		assert_eq!(get_gamma_abc_g1_2(), pvk.vk.gamma_abc_g1[2]);
		assert_eq!(get_gamma_abc_g1_3(), pvk.vk.gamma_abc_g1[3]);
		assert_eq!(get_gamma_abc_g1_4(), pvk.vk.gamma_abc_g1[4]);
		assert_eq!(get_gamma_abc_g1_5(), pvk.vk.gamma_abc_g1[5]);
		assert_eq!(get_gamma_abc_g1_6(), pvk.vk.gamma_abc_g1[6]);
		assert_eq!(get_gamma_abc_g1_7(), pvk.vk.gamma_abc_g1[7]);
		assert_eq!(get_gamma_g2_neg_pc_0(), pvk.gamma_g2_neg_pc.ell_coeffs[0]);
		assert_eq!(get_gamma_g2_neg_pc_1(), pvk.gamma_g2_neg_pc.ell_coeffs[1]);
		assert_eq!(get_gamma_g2_neg_pc_2(), pvk.gamma_g2_neg_pc.ell_coeffs[2]);
		assert_eq!(get_gamma_g2_neg_pc_3(), pvk.gamma_g2_neg_pc.ell_coeffs[3]);
		assert_eq!(get_gamma_g2_neg_pc_4(), pvk.gamma_g2_neg_pc.ell_coeffs[4]);
		assert_eq!(get_gamma_g2_neg_pc_5(), pvk.gamma_g2_neg_pc.ell_coeffs[5]);
		assert_eq!(get_gamma_g2_neg_pc_6(), pvk.gamma_g2_neg_pc.ell_coeffs[6]);
		assert_eq!(get_gamma_g2_neg_pc_7(), pvk.gamma_g2_neg_pc.ell_coeffs[7]);
		assert_eq!(get_gamma_g2_neg_pc_8(), pvk.gamma_g2_neg_pc.ell_coeffs[8]);
		assert_eq!(get_gamma_g2_neg_pc_9(), pvk.gamma_g2_neg_pc.ell_coeffs[9]);
		assert_eq!(get_gamma_g2_neg_pc_10(), pvk.gamma_g2_neg_pc.ell_coeffs[10]);
		assert_eq!(get_gamma_g2_neg_pc_11(), pvk.gamma_g2_neg_pc.ell_coeffs[11]);
		assert_eq!(get_gamma_g2_neg_pc_12(), pvk.gamma_g2_neg_pc.ell_coeffs[12]);
		assert_eq!(get_gamma_g2_neg_pc_13(), pvk.gamma_g2_neg_pc.ell_coeffs[13]);
		assert_eq!(get_gamma_g2_neg_pc_14(), pvk.gamma_g2_neg_pc.ell_coeffs[14]);
		assert_eq!(get_gamma_g2_neg_pc_15(), pvk.gamma_g2_neg_pc.ell_coeffs[15]);
		assert_eq!(get_gamma_g2_neg_pc_16(), pvk.gamma_g2_neg_pc.ell_coeffs[16]);
		assert_eq!(get_gamma_g2_neg_pc_17(), pvk.gamma_g2_neg_pc.ell_coeffs[17]);
		assert_eq!(get_gamma_g2_neg_pc_18(), pvk.gamma_g2_neg_pc.ell_coeffs[18]);
		assert_eq!(get_gamma_g2_neg_pc_19(), pvk.gamma_g2_neg_pc.ell_coeffs[19]);
		assert_eq!(get_gamma_g2_neg_pc_20(), pvk.gamma_g2_neg_pc.ell_coeffs[20]);
		assert_eq!(get_gamma_g2_neg_pc_21(), pvk.gamma_g2_neg_pc.ell_coeffs[21]);
		assert_eq!(get_gamma_g2_neg_pc_22(), pvk.gamma_g2_neg_pc.ell_coeffs[22]);
		assert_eq!(get_gamma_g2_neg_pc_23(), pvk.gamma_g2_neg_pc.ell_coeffs[23]);
		assert_eq!(get_gamma_g2_neg_pc_24(), pvk.gamma_g2_neg_pc.ell_coeffs[24]);
		assert_eq!(get_gamma_g2_neg_pc_25(), pvk.gamma_g2_neg_pc.ell_coeffs[25]);
		assert_eq!(get_gamma_g2_neg_pc_26(), pvk.gamma_g2_neg_pc.ell_coeffs[26]);
		assert_eq!(get_gamma_g2_neg_pc_27(), pvk.gamma_g2_neg_pc.ell_coeffs[27]);
		assert_eq!(get_gamma_g2_neg_pc_28(), pvk.gamma_g2_neg_pc.ell_coeffs[28]);
		assert_eq!(get_gamma_g2_neg_pc_29(), pvk.gamma_g2_neg_pc.ell_coeffs[29]);
		assert_eq!(get_gamma_g2_neg_pc_30(), pvk.gamma_g2_neg_pc.ell_coeffs[30]);
		assert_eq!(get_gamma_g2_neg_pc_31(), pvk.gamma_g2_neg_pc.ell_coeffs[31]);
		assert_eq!(get_gamma_g2_neg_pc_32(), pvk.gamma_g2_neg_pc.ell_coeffs[32]);
		assert_eq!(get_gamma_g2_neg_pc_33(), pvk.gamma_g2_neg_pc.ell_coeffs[33]);
		assert_eq!(get_gamma_g2_neg_pc_34(), pvk.gamma_g2_neg_pc.ell_coeffs[34]);
		assert_eq!(get_gamma_g2_neg_pc_35(), pvk.gamma_g2_neg_pc.ell_coeffs[35]);
		assert_eq!(get_gamma_g2_neg_pc_36(), pvk.gamma_g2_neg_pc.ell_coeffs[36]);
		assert_eq!(get_gamma_g2_neg_pc_37(), pvk.gamma_g2_neg_pc.ell_coeffs[37]);
		assert_eq!(get_gamma_g2_neg_pc_38(), pvk.gamma_g2_neg_pc.ell_coeffs[38]);
		assert_eq!(get_gamma_g2_neg_pc_39(), pvk.gamma_g2_neg_pc.ell_coeffs[39]);
		assert_eq!(get_gamma_g2_neg_pc_40(), pvk.gamma_g2_neg_pc.ell_coeffs[40]);
		assert_eq!(get_gamma_g2_neg_pc_41(), pvk.gamma_g2_neg_pc.ell_coeffs[41]);
		assert_eq!(get_gamma_g2_neg_pc_42(), pvk.gamma_g2_neg_pc.ell_coeffs[42]);
		assert_eq!(get_gamma_g2_neg_pc_43(), pvk.gamma_g2_neg_pc.ell_coeffs[43]);
		assert_eq!(get_gamma_g2_neg_pc_44(), pvk.gamma_g2_neg_pc.ell_coeffs[44]);
		assert_eq!(get_gamma_g2_neg_pc_45(), pvk.gamma_g2_neg_pc.ell_coeffs[45]);
		assert_eq!(get_gamma_g2_neg_pc_46(), pvk.gamma_g2_neg_pc.ell_coeffs[46]);
		assert_eq!(get_gamma_g2_neg_pc_47(), pvk.gamma_g2_neg_pc.ell_coeffs[47]);
		assert_eq!(get_gamma_g2_neg_pc_48(), pvk.gamma_g2_neg_pc.ell_coeffs[48]);
		assert_eq!(get_gamma_g2_neg_pc_49(), pvk.gamma_g2_neg_pc.ell_coeffs[49]);
		assert_eq!(get_gamma_g2_neg_pc_50(), pvk.gamma_g2_neg_pc.ell_coeffs[50]);
		assert_eq!(get_gamma_g2_neg_pc_51(), pvk.gamma_g2_neg_pc.ell_coeffs[51]);
		assert_eq!(get_gamma_g2_neg_pc_52(), pvk.gamma_g2_neg_pc.ell_coeffs[52]);
		assert_eq!(get_gamma_g2_neg_pc_53(), pvk.gamma_g2_neg_pc.ell_coeffs[53]);
		assert_eq!(get_gamma_g2_neg_pc_54(), pvk.gamma_g2_neg_pc.ell_coeffs[54]);
		assert_eq!(get_gamma_g2_neg_pc_55(), pvk.gamma_g2_neg_pc.ell_coeffs[55]);
		assert_eq!(get_gamma_g2_neg_pc_56(), pvk.gamma_g2_neg_pc.ell_coeffs[56]);
		assert_eq!(get_gamma_g2_neg_pc_57(), pvk.gamma_g2_neg_pc.ell_coeffs[57]);
		assert_eq!(get_gamma_g2_neg_pc_58(), pvk.gamma_g2_neg_pc.ell_coeffs[58]);
		assert_eq!(get_gamma_g2_neg_pc_59(), pvk.gamma_g2_neg_pc.ell_coeffs[59]);
		assert_eq!(get_gamma_g2_neg_pc_60(), pvk.gamma_g2_neg_pc.ell_coeffs[60]);
		assert_eq!(get_gamma_g2_neg_pc_61(), pvk.gamma_g2_neg_pc.ell_coeffs[61]);
		assert_eq!(get_gamma_g2_neg_pc_62(), pvk.gamma_g2_neg_pc.ell_coeffs[62]);
		assert_eq!(get_gamma_g2_neg_pc_63(), pvk.gamma_g2_neg_pc.ell_coeffs[63]);
		assert_eq!(get_gamma_g2_neg_pc_64(), pvk.gamma_g2_neg_pc.ell_coeffs[64]);
		assert_eq!(get_gamma_g2_neg_pc_65(), pvk.gamma_g2_neg_pc.ell_coeffs[65]);
		assert_eq!(get_gamma_g2_neg_pc_66(), pvk.gamma_g2_neg_pc.ell_coeffs[66]);
		assert_eq!(get_gamma_g2_neg_pc_67(), pvk.gamma_g2_neg_pc.ell_coeffs[67]);
		assert_eq!(get_gamma_g2_neg_pc_68(), pvk.gamma_g2_neg_pc.ell_coeffs[68]);
		assert_eq!(get_gamma_g2_neg_pc_69(), pvk.gamma_g2_neg_pc.ell_coeffs[69]);
		assert_eq!(get_gamma_g2_neg_pc_70(), pvk.gamma_g2_neg_pc.ell_coeffs[70]);
		assert_eq!(get_gamma_g2_neg_pc_71(), pvk.gamma_g2_neg_pc.ell_coeffs[71]);
		assert_eq!(get_gamma_g2_neg_pc_72(), pvk.gamma_g2_neg_pc.ell_coeffs[72]);
		assert_eq!(get_gamma_g2_neg_pc_73(), pvk.gamma_g2_neg_pc.ell_coeffs[73]);
		assert_eq!(get_gamma_g2_neg_pc_74(), pvk.gamma_g2_neg_pc.ell_coeffs[74]);
		assert_eq!(get_gamma_g2_neg_pc_75(), pvk.gamma_g2_neg_pc.ell_coeffs[75]);
		assert_eq!(get_gamma_g2_neg_pc_76(), pvk.gamma_g2_neg_pc.ell_coeffs[76]);
		assert_eq!(get_gamma_g2_neg_pc_77(), pvk.gamma_g2_neg_pc.ell_coeffs[77]);
		assert_eq!(get_gamma_g2_neg_pc_78(), pvk.gamma_g2_neg_pc.ell_coeffs[78]);
		assert_eq!(get_gamma_g2_neg_pc_79(), pvk.gamma_g2_neg_pc.ell_coeffs[79]);
		assert_eq!(get_gamma_g2_neg_pc_80(), pvk.gamma_g2_neg_pc.ell_coeffs[80]);
		assert_eq!(get_gamma_g2_neg_pc_81(), pvk.gamma_g2_neg_pc.ell_coeffs[81]);
		assert_eq!(get_gamma_g2_neg_pc_82(), pvk.gamma_g2_neg_pc.ell_coeffs[82]);
		assert_eq!(get_gamma_g2_neg_pc_83(), pvk.gamma_g2_neg_pc.ell_coeffs[83]);
		assert_eq!(get_gamma_g2_neg_pc_84(), pvk.gamma_g2_neg_pc.ell_coeffs[84]);
		assert_eq!(get_gamma_g2_neg_pc_85(), pvk.gamma_g2_neg_pc.ell_coeffs[85]);
		assert_eq!(get_gamma_g2_neg_pc_86(), pvk.gamma_g2_neg_pc.ell_coeffs[86]);
		assert_eq!(get_gamma_g2_neg_pc_87(), pvk.gamma_g2_neg_pc.ell_coeffs[87]);
		assert_eq!(get_gamma_g2_neg_pc_88(), pvk.gamma_g2_neg_pc.ell_coeffs[88]);
		assert_eq!(get_gamma_g2_neg_pc_89(), pvk.gamma_g2_neg_pc.ell_coeffs[89]);
		assert_eq!(get_gamma_g2_neg_pc_90(), pvk.gamma_g2_neg_pc.ell_coeffs[90]);
		assert_eq!(get_delta_g2_neg_pc_0(), pvk.delta_g2_neg_pc.ell_coeffs[0]);
		assert_eq!(get_delta_g2_neg_pc_1(), pvk.delta_g2_neg_pc.ell_coeffs[1]);
		assert_eq!(get_delta_g2_neg_pc_2(), pvk.delta_g2_neg_pc.ell_coeffs[2]);
		assert_eq!(get_delta_g2_neg_pc_3(), pvk.delta_g2_neg_pc.ell_coeffs[3]);
		assert_eq!(get_delta_g2_neg_pc_4(), pvk.delta_g2_neg_pc.ell_coeffs[4]);
		assert_eq!(get_delta_g2_neg_pc_5(), pvk.delta_g2_neg_pc.ell_coeffs[5]);
		assert_eq!(get_delta_g2_neg_pc_6(), pvk.delta_g2_neg_pc.ell_coeffs[6]);
		assert_eq!(get_delta_g2_neg_pc_7(), pvk.delta_g2_neg_pc.ell_coeffs[7]);
		assert_eq!(get_delta_g2_neg_pc_8(), pvk.delta_g2_neg_pc.ell_coeffs[8]);
		assert_eq!(get_delta_g2_neg_pc_9(), pvk.delta_g2_neg_pc.ell_coeffs[9]);
		assert_eq!(get_delta_g2_neg_pc_10(), pvk.delta_g2_neg_pc.ell_coeffs[10]);
		assert_eq!(get_delta_g2_neg_pc_11(), pvk.delta_g2_neg_pc.ell_coeffs[11]);
		assert_eq!(get_delta_g2_neg_pc_12(), pvk.delta_g2_neg_pc.ell_coeffs[12]);
		assert_eq!(get_delta_g2_neg_pc_13(), pvk.delta_g2_neg_pc.ell_coeffs[13]);
		assert_eq!(get_delta_g2_neg_pc_14(), pvk.delta_g2_neg_pc.ell_coeffs[14]);
		assert_eq!(get_delta_g2_neg_pc_15(), pvk.delta_g2_neg_pc.ell_coeffs[15]);
		assert_eq!(get_delta_g2_neg_pc_16(), pvk.delta_g2_neg_pc.ell_coeffs[16]);
		assert_eq!(get_delta_g2_neg_pc_17(), pvk.delta_g2_neg_pc.ell_coeffs[17]);
		assert_eq!(get_delta_g2_neg_pc_18(), pvk.delta_g2_neg_pc.ell_coeffs[18]);
		assert_eq!(get_delta_g2_neg_pc_19(), pvk.delta_g2_neg_pc.ell_coeffs[19]);
		assert_eq!(get_delta_g2_neg_pc_20(), pvk.delta_g2_neg_pc.ell_coeffs[20]);
		assert_eq!(get_delta_g2_neg_pc_21(), pvk.delta_g2_neg_pc.ell_coeffs[21]);
		assert_eq!(get_delta_g2_neg_pc_22(), pvk.delta_g2_neg_pc.ell_coeffs[22]);
		assert_eq!(get_delta_g2_neg_pc_23(), pvk.delta_g2_neg_pc.ell_coeffs[23]);
		assert_eq!(get_delta_g2_neg_pc_24(), pvk.delta_g2_neg_pc.ell_coeffs[24]);
		assert_eq!(get_delta_g2_neg_pc_25(), pvk.delta_g2_neg_pc.ell_coeffs[25]);
		assert_eq!(get_delta_g2_neg_pc_26(), pvk.delta_g2_neg_pc.ell_coeffs[26]);
		assert_eq!(get_delta_g2_neg_pc_27(), pvk.delta_g2_neg_pc.ell_coeffs[27]);
		assert_eq!(get_delta_g2_neg_pc_28(), pvk.delta_g2_neg_pc.ell_coeffs[28]);
		assert_eq!(get_delta_g2_neg_pc_29(), pvk.delta_g2_neg_pc.ell_coeffs[29]);
		assert_eq!(get_delta_g2_neg_pc_30(), pvk.delta_g2_neg_pc.ell_coeffs[30]);
		assert_eq!(get_delta_g2_neg_pc_31(), pvk.delta_g2_neg_pc.ell_coeffs[31]);
		assert_eq!(get_delta_g2_neg_pc_32(), pvk.delta_g2_neg_pc.ell_coeffs[32]);
		assert_eq!(get_delta_g2_neg_pc_33(), pvk.delta_g2_neg_pc.ell_coeffs[33]);
		assert_eq!(get_delta_g2_neg_pc_34(), pvk.delta_g2_neg_pc.ell_coeffs[34]);
		assert_eq!(get_delta_g2_neg_pc_35(), pvk.delta_g2_neg_pc.ell_coeffs[35]);
		assert_eq!(get_delta_g2_neg_pc_36(), pvk.delta_g2_neg_pc.ell_coeffs[36]);
		assert_eq!(get_delta_g2_neg_pc_37(), pvk.delta_g2_neg_pc.ell_coeffs[37]);
		assert_eq!(get_delta_g2_neg_pc_38(), pvk.delta_g2_neg_pc.ell_coeffs[38]);
		assert_eq!(get_delta_g2_neg_pc_39(), pvk.delta_g2_neg_pc.ell_coeffs[39]);
		assert_eq!(get_delta_g2_neg_pc_40(), pvk.delta_g2_neg_pc.ell_coeffs[40]);
		assert_eq!(get_delta_g2_neg_pc_41(), pvk.delta_g2_neg_pc.ell_coeffs[41]);
		assert_eq!(get_delta_g2_neg_pc_42(), pvk.delta_g2_neg_pc.ell_coeffs[42]);
		assert_eq!(get_delta_g2_neg_pc_43(), pvk.delta_g2_neg_pc.ell_coeffs[43]);
		assert_eq!(get_delta_g2_neg_pc_44(), pvk.delta_g2_neg_pc.ell_coeffs[44]);
		assert_eq!(get_delta_g2_neg_pc_45(), pvk.delta_g2_neg_pc.ell_coeffs[45]);
		assert_eq!(get_delta_g2_neg_pc_46(), pvk.delta_g2_neg_pc.ell_coeffs[46]);
		assert_eq!(get_delta_g2_neg_pc_47(), pvk.delta_g2_neg_pc.ell_coeffs[47]);
		assert_eq!(get_delta_g2_neg_pc_48(), pvk.delta_g2_neg_pc.ell_coeffs[48]);
		assert_eq!(get_delta_g2_neg_pc_49(), pvk.delta_g2_neg_pc.ell_coeffs[49]);
		assert_eq!(get_delta_g2_neg_pc_50(), pvk.delta_g2_neg_pc.ell_coeffs[50]);
		assert_eq!(get_delta_g2_neg_pc_51(), pvk.delta_g2_neg_pc.ell_coeffs[51]);
		assert_eq!(get_delta_g2_neg_pc_52(), pvk.delta_g2_neg_pc.ell_coeffs[52]);
		assert_eq!(get_delta_g2_neg_pc_53(), pvk.delta_g2_neg_pc.ell_coeffs[53]);
		assert_eq!(get_delta_g2_neg_pc_54(), pvk.delta_g2_neg_pc.ell_coeffs[54]);
		assert_eq!(get_delta_g2_neg_pc_55(), pvk.delta_g2_neg_pc.ell_coeffs[55]);
		assert_eq!(get_delta_g2_neg_pc_56(), pvk.delta_g2_neg_pc.ell_coeffs[56]);
		assert_eq!(get_delta_g2_neg_pc_57(), pvk.delta_g2_neg_pc.ell_coeffs[57]);
		assert_eq!(get_delta_g2_neg_pc_58(), pvk.delta_g2_neg_pc.ell_coeffs[58]);
		assert_eq!(get_delta_g2_neg_pc_59(), pvk.delta_g2_neg_pc.ell_coeffs[59]);
		assert_eq!(get_delta_g2_neg_pc_60(), pvk.delta_g2_neg_pc.ell_coeffs[60]);
		assert_eq!(get_delta_g2_neg_pc_61(), pvk.delta_g2_neg_pc.ell_coeffs[61]);
		assert_eq!(get_delta_g2_neg_pc_62(), pvk.delta_g2_neg_pc.ell_coeffs[62]);
		assert_eq!(get_delta_g2_neg_pc_63(), pvk.delta_g2_neg_pc.ell_coeffs[63]);
		assert_eq!(get_delta_g2_neg_pc_64(), pvk.delta_g2_neg_pc.ell_coeffs[64]);
		assert_eq!(get_delta_g2_neg_pc_65(), pvk.delta_g2_neg_pc.ell_coeffs[65]);
		assert_eq!(get_delta_g2_neg_pc_66(), pvk.delta_g2_neg_pc.ell_coeffs[66]);
		assert_eq!(get_delta_g2_neg_pc_67(), pvk.delta_g2_neg_pc.ell_coeffs[67]);
		assert_eq!(get_delta_g2_neg_pc_68(), pvk.delta_g2_neg_pc.ell_coeffs[68]);
		assert_eq!(get_delta_g2_neg_pc_69(), pvk.delta_g2_neg_pc.ell_coeffs[69]);
		assert_eq!(get_delta_g2_neg_pc_70(), pvk.delta_g2_neg_pc.ell_coeffs[70]);
		assert_eq!(get_delta_g2_neg_pc_71(), pvk.delta_g2_neg_pc.ell_coeffs[71]);
		assert_eq!(get_delta_g2_neg_pc_72(), pvk.delta_g2_neg_pc.ell_coeffs[72]);
		assert_eq!(get_delta_g2_neg_pc_73(), pvk.delta_g2_neg_pc.ell_coeffs[73]);
		assert_eq!(get_delta_g2_neg_pc_74(), pvk.delta_g2_neg_pc.ell_coeffs[74]);
		assert_eq!(get_delta_g2_neg_pc_75(), pvk.delta_g2_neg_pc.ell_coeffs[75]);
		assert_eq!(get_delta_g2_neg_pc_76(), pvk.delta_g2_neg_pc.ell_coeffs[76]);
		assert_eq!(get_delta_g2_neg_pc_77(), pvk.delta_g2_neg_pc.ell_coeffs[77]);
		assert_eq!(get_delta_g2_neg_pc_78(), pvk.delta_g2_neg_pc.ell_coeffs[78]);
		assert_eq!(get_delta_g2_neg_pc_79(), pvk.delta_g2_neg_pc.ell_coeffs[79]);
		assert_eq!(get_delta_g2_neg_pc_80(), pvk.delta_g2_neg_pc.ell_coeffs[80]);
		assert_eq!(get_delta_g2_neg_pc_81(), pvk.delta_g2_neg_pc.ell_coeffs[81]);
		assert_eq!(get_delta_g2_neg_pc_82(), pvk.delta_g2_neg_pc.ell_coeffs[82]);
		assert_eq!(get_delta_g2_neg_pc_83(), pvk.delta_g2_neg_pc.ell_coeffs[83]);
		assert_eq!(get_delta_g2_neg_pc_84(), pvk.delta_g2_neg_pc.ell_coeffs[84]);
		assert_eq!(get_delta_g2_neg_pc_85(), pvk.delta_g2_neg_pc.ell_coeffs[85]);
		assert_eq!(get_delta_g2_neg_pc_86(), pvk.delta_g2_neg_pc.ell_coeffs[86]);
		assert_eq!(get_delta_g2_neg_pc_87(), pvk.delta_g2_neg_pc.ell_coeffs[87]);
		assert_eq!(get_delta_g2_neg_pc_88(), pvk.delta_g2_neg_pc.ell_coeffs[88]);
		assert_eq!(get_delta_g2_neg_pc_89(), pvk.delta_g2_neg_pc.ell_coeffs[89]);
		assert_eq!(get_delta_g2_neg_pc_90(), pvk.delta_g2_neg_pc.ell_coeffs[90]);
	}

	#[test]
	fn verification_test() -> Result<()> {
		let pvk_unprepped = get_pvk_from_bytes_254()?;
		let pvk = prepare_verifying_key(&pvk_unprepped);
		let proof = get_proof_from_bytes_254()?;

		let public_inputs = get_public_inputs_from_bytes_254()?;

		let _res = verify_proof(&pvk, &proof, &public_inputs[..]);

		Ok(())
	}

	use ark_ff::CubicExtField;
	#[test]
	#[ignore]
	fn fe_test_offchain() -> Result<()> {
		let pvk_unprepped = get_pvk_from_bytes_254()?;
		let pvk = prepare_verifying_key(&pvk_unprepped);
		let proof = get_proof_from_bytes_254()?;

		let public_inputs = get_public_inputs_from_bytes_254()?;

		let prepared_inputs = prepare_inputs(&pvk, &public_inputs).unwrap();

		let miller_output =
			<ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::miller_loop(
				[
					(proof.a.into(), proof.b.into()),
					(
						(prepared_inputs).into_affine().into(),
						pvk.gamma_g2_neg_pc.clone(),
					),
					(proof.c.into(), pvk.delta_g2_neg_pc.clone()),
				]
				.iter(),
			);
		//starting result
		let f = miller_output;

		//library result for reference
		let res_origin = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::final_exponentiation(&f).unwrap();
		let res_custom = final_exponentiation_custom(&f).unwrap();
		assert_eq!(res_origin, res_custom);
		let res_processor = final_exponentiation_test_processor(&f).unwrap();
		assert_eq!(res_origin, res_processor);

		// println!("{:?}", res_origin);
		//println!("{:?}", pvk.alpha_g1_beta_g2.c0[0]);
		// let pvk_hard_coded =
		// 	QuadExtField::<ark_ff::Fp12ParamsWrapper<ark_bn254::Fq12Parameters>>::new(
		// 		CubicExtField::<ark_ff::Fp6ParamsWrapper<ark_bn254::Fq6Parameters>>::new(
		// 			QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
		// 				ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
		// 					17214827553771518527,
		// 					8103811577513533309,
		// 					5824106868827698446,
		// 					538393706883776885,
		// 				])),
		// 				ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
		// 					1747766986087995073,
		// 					17030008964085198309,
		// 					14711893862670036801,
		// 					1251847809326396116,
		// 				])),
		// 			),
		// 			QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
		// 				ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
		// 					8670468519825929670,
		// 					6774311001955862070,
		// 					14503208649103997400,
		// 					2739832133422703605,
		// 				])),
		// 				ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
		// 					3403041057055849213,
		// 					5589831403557161118,
		// 					11353848742706634430,
		// 					2079335176187258289,
		// 				])),
		// 			),
		// 			QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
		// 				ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
		// 					5700889876348332023,
		// 					5164370052034384707,
		// 					11026397386690668186,
		// 					1430638717145074535,
		// 				])),
		// 				ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
		// 					14585014708672115679,
		// 					10557724701831733650,
		// 					11346225797950201897,
		// 					163817071525994422,
		// 				])),
		// 			),
		// 		),
		// 		CubicExtField::<ark_ff::Fp6ParamsWrapper<ark_bn254::Fq6Parameters>>::new(
		// 			QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
		// 				ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
		// 					184137068633880152,
		// 					15666126431488555624,
		// 					15896723566730834541,
		// 					327734949610890862,
		// 				])),
		// 				ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
		// 					5217626969957908428,
		// 					13857069499728575185,
		// 					16747932664762117536,
		// 					1511015936345776210,
		// 				])),
		// 			),
		// 			QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
		// 				ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
		// 					4044920854921985794,
		// 					16524891583600629150,
		// 					17295166532143782492,
		// 					1552849265734776570,
		// 				])),
		// 				ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
		// 					7380548997166592537,
		// 					191847233093951225,
		// 					8211711349787187541,
		// 					2939180299531928202,
		// 				])),
		// 			),
		// 			QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>::new(
		// 				ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
		// 					12782511908424732804,
		// 					9912157266960376288,
		// 					15239332730960188312,
		// 					1839595783782490417,
		// 				])),
		// 				ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([
		// 					1680073062438571392,
		// 					2800534229562584231,
		// 					800746447625002697,
		// 					1128810302869726976,
		// 				])),
		// 			),
		// 		),
		// 	);
		// assert_eq!(pvk_hard_coded, pvk.alpha_g1_beta_g2);

		assert_eq!(res_origin, pvk.alpha_g1_beta_g2);
		Ok(())
	}

	#[allow(clippy::let_and_return)]
	fn final_exponentiation_custom(
		f: &<ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk,
	) -> Option<<ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk> {
		//adapted from ark_ec bn254
		//executes fe_instructions alongside reference implementation and
		//asserts after every step

		//from original repo:
		// Easy part: result = elt^((q^6-1)*(q^2+1)).
		// Follows, e.g., Beuchat et al page 9, by computing result as follows:
		//   elt^((q^6-1)*(q^2+1)) = (conj(elt) * elt^(-1))^(q^2+1)
		let mut instruction_order = Vec::new();
		/*
			* ------------------------- init -------------------------
			*instruction 0
			*/
		let mut account_struct = FinalExpBytes::new();
		let mut account_struct1 = FinalExpBytes::new();

		// f1 = r.conjugate() = f^(p^6)
		let mut f1 = *f;
		parse_f_to_bytes(*f, &mut account_struct.f1_r_range_s);
		parse_f_to_bytes(*f, &mut account_struct1.f_f2_range_s);

		assert_eq!(
			f1,
			parse_f_from_bytes(&account_struct.f1_r_range_s),
			"0 failed"
		);
		assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
		assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
		assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
		assert_eq!(account_struct1.y6_range, account_struct.y6_range);
		_process_instruction(&mut account_struct1, 0);

		let reference_f = f.clone();
		account_struct.f_f2_range_s = account_struct.f1_r_range_s.clone();
		/*
			* ------------------------- conjugate -------------------------
			*instruction 0
			*/
		f1.conjugate();
		conjugate_wrapper(&mut account_struct.f1_r_range_s);
		instruction_order.push(0);
		assert_eq!(
			f1,
			parse_f_from_bytes(&account_struct.f1_r_range_s),
			"1 failed"
		);

		/*
			*
			* ------------------------- Inverse -------------------------
			* instruction 1
			*/
		instruction_order.push(1);
		assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
		assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
		assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
		assert_eq!(account_struct1.y6_range, account_struct.y6_range);
		_process_instruction(&mut account_struct1, 1);
		custom_f_inverse_1(
			&account_struct.f_f2_range_s,
			&mut account_struct.cubic_range_1_s,
		);

		//instruction 2 ---------------------------------------------
		instruction_order.push(2);
		assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
		assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
		assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
		assert_eq!(account_struct1.y6_range, account_struct.y6_range);
		_process_instruction(&mut account_struct1, 2);

		custom_f_inverse_2(
			&account_struct.f_f2_range_s,
			&mut account_struct.cubic_range_0_s,
			&account_struct.cubic_range_1_s,
		);

		//instruction 3 ---------------------------------------------
		instruction_order.push(3);
		assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
		assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
		assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
		assert_eq!(account_struct1.y6_range, account_struct.y6_range);
		_process_instruction(&mut account_struct1, 3);

		custom_cubic_inverse_1(
			&account_struct.cubic_range_0_s,
			&mut account_struct.quad_range_0_s,
			&mut account_struct.quad_range_1_s,
			&mut account_struct.quad_range_2_s,
			&mut account_struct.quad_range_3_s,
		);

		//instruction 4 ---------------------------------------------
		instruction_order.push(4);
		assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
		assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
		assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
		assert_eq!(account_struct1.y6_range, account_struct.y6_range);
		_process_instruction(&mut account_struct1, 4);

		//quad inverse is part of cubic Inverse
		custom_quadratic_fp256_inverse_1(
			&account_struct.quad_range_3_s,
			&mut account_struct.fp384_range_s,
		);

		//instruction 5 ---------------------------------------------
		instruction_order.push(5);
		assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
		assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
		assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
		assert_eq!(account_struct1.y6_range, account_struct.y6_range);
		_process_instruction(&mut account_struct1, 5);

		custom_quadratic_fp256_inverse_2(
			&mut account_struct.quad_range_3_s,
			&account_struct.fp384_range_s,
		);

		//instruction 6 ---------------------------------------------
		instruction_order.push(6);
		assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
		assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
		assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
		assert_eq!(account_struct1.y6_range, account_struct.y6_range);
		_process_instruction(&mut account_struct1, 6);

		custom_cubic_inverse_2(
			&mut account_struct.cubic_range_0_s,
			&account_struct.quad_range_0_s,
			&account_struct.quad_range_1_s,
			&account_struct.quad_range_2_s,
			&account_struct.quad_range_3_s,
		);

		//instruction 7 ---------------------------------------------
		instruction_order.push(7);
		assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
		assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
		assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
		assert_eq!(account_struct1.y6_range, account_struct.y6_range);
		_process_instruction(&mut account_struct1, 7);

		custom_f_inverse_3(
			&mut account_struct.cubic_range_1_s,
			&account_struct.cubic_range_0_s,
			&account_struct.f_f2_range_s,
		);

		//instruction 8 ---------------------------------------------
		instruction_order.push(8);
		assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
		assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
		assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
		assert_eq!(account_struct1.y6_range, account_struct.y6_range);
		_process_instruction(&mut account_struct1, 8);

		custom_f_inverse_4(
			&mut account_struct.cubic_range_0_s,
			&account_struct.f_f2_range_s,
		);

		//instruction 9 ---------------------------------------------
		instruction_order.push(9);
		assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
		assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
		assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
		assert_eq!(account_struct1.y6_range, account_struct.y6_range);
		_process_instruction(&mut account_struct1, 9);

		custom_f_inverse_5(
			&account_struct.cubic_range_0_s,
			&account_struct.cubic_range_1_s,
			&mut account_struct.f_f2_range_s,
		);

		assert_eq!(
			reference_f.inverse().unwrap(),
			parse_f_from_bytes(&account_struct.f_f2_range_s),
			"f inverse failed"
		);
		assert_eq!(f1, parse_f_from_bytes(&account_struct.f1_r_range_s));

		f.inverse().map(|mut f2| {
			/*
				*
				*
				* ------------------------- mul -------------------------
				*
				*
				*/
			//from original repo
			// f2 = f^(-1);
			// r = f^(p^6 - 1)

			let mut r = f1 * &f2;

			assert_eq!(f2, parse_f_from_bytes(&account_struct.f_f2_range_s));
			assert_eq!(f1, parse_f_from_bytes(&account_struct.f1_r_range_s));
			//instruction 10 ---------------------------------------------
			instruction_order.push(10);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 10);

			mul_assign_1_2(
				&account_struct.f1_r_range_s,
				&account_struct.f_f2_range_s,
				&mut account_struct.cubic_range_0_s,
				&mut account_struct.cubic_range_1_s,
			);

			//instruction 11 ---------------------------------------------
			instruction_order.push(11);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 11);

			mul_assign_3_4_5(
				&account_struct.f_f2_range_s,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s,
				&mut account_struct.f1_r_range_s,
			);

			assert_eq!(
				r,
				parse_f_from_bytes(&account_struct.f1_r_range_s),
				"f mulassign failed"
			);

			/*
				*
				*
				* ------------------------- assign -------------------------
				*
				*
				*/
			//from original repo
			// f2 = f^(p^6 - 1)
			f2 = r;
			//instruction 12 ---------------------------------------------
			instruction_order.push(12);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 12);

			account_struct.f_f2_range_s = account_struct.f1_r_range_s.clone();
			assert_eq!(f2, parse_f_from_bytes(&account_struct.f_f2_range_s));

			/*
				*
				*
				* ------------------------- frobenius_map(2) -------------------------
				*
				*
				*/
			//from original repo
			// r = f^((p^6 - 1)(p^2))
			r.frobenius_map(2);

			//instruction 13 ---------------------------------------------
			instruction_order.push(13);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 13);

			custom_frobenius_map_2(&mut account_struct.f1_r_range_s);

			assert_eq!(r, parse_f_from_bytes(&account_struct.f1_r_range_s));

			/*
				*
				*
				* ------------------------- mulassign -------------------------
				*
				*
				*/
			//from original repo
			// r = f^((p^6 - 1)(p^2) + (p^6 - 1))
			// r = f^((p^6 - 1)(p^2 + 1))
			r *= &f2;
			//f2 last used here
			//instruction 10 ---------------------------------------------
			instruction_order.push(10);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 10);

			mul_assign_1_2(
				&account_struct.f1_r_range_s,
				&account_struct.f_f2_range_s,
				&mut account_struct.cubic_range_0_s,
				&mut account_struct.cubic_range_1_s,
			);

			//instruction 11 ---------------------------------------------
			instruction_order.push(11);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 11);

			mul_assign_3_4_5(
				&account_struct.f_f2_range_s,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s,
				&mut account_struct.f1_r_range_s,
			);

			assert_eq!(
				r,
				parse_f_from_bytes(&account_struct.f1_r_range_s),
				"f mulassign failed"
			);

			/*
				*
				*
				* ------------------------- exp_by_neg_x(r) -------------------------
				*
				*
				*/
			//from original repo
			// Hard part follows Laura Fuentes-Castaneda et al. "Faster hashing to G2"
			// by computing:
			//
			// result = elt^(q^3 * (12*z^3 + 6z^2 + 4z - 1) +
			//               q^2 * (12*z^3 + 6z^2 + 6z) +
			//               q   * (12*z^3 + 6z^2 + 4z) +
			//               1   * (12*z^3 + 12z^2 + 6z + 1))
			// which equals
			//
			// result = elt^( 2z * ( 6z^2 + 3z + 1 ) * (q^4 - q^2 + 1)/r ).

			let y0 = exp_by_neg_x(r);

			//init
			//instruction 14 ---------------------------------------------
			instruction_order.push(14);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 14);

			account_struct.i_range_s = account_struct.f1_r_range_s.clone();
			conjugate_wrapper(&mut account_struct.i_range_s);
			account_struct.y0_range_s = account_struct.f1_r_range_s.clone();

			for i in 1..63 {
				if i == 1 {
					assert_eq!(account_struct.y0_range_s, account_struct.f1_r_range_s);
				}
				println!("i {}", i);
				//cyclotomic_exp
				//instruction 15 ---------------------------------------------
				instruction_order.push(15);
				assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
				assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
				assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
				assert_eq!(account_struct1.y6_range, account_struct.y6_range);
				_process_instruction(&mut account_struct1, 15);

				custom_cyclotomic_square_in_place(&mut account_struct.y0_range_s);

				if NAF_VEC[i] != 0 {
					if NAF_VEC[i] > 0 {
						//println!("if i {}", i);
						//instruction 16 ---------------------------------------------
						instruction_order.push(16);
						assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
						assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
						assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
						assert_eq!(account_struct1.y6_range, account_struct.y6_range);
						_process_instruction(&mut account_struct1, 16);
						mul_assign_1_2(
							&account_struct.y0_range_s,
							&account_struct.f1_r_range_s,
							&mut account_struct.cubic_range_0_s,
							&mut account_struct.cubic_range_1_s,
						);

						//instruction 17 ---------------------------------------------
						instruction_order.push(17);
						assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
						assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
						assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
						assert_eq!(account_struct1.y6_range, account_struct.y6_range);
						_process_instruction(&mut account_struct1, 17);

						mul_assign_3_4_5(
							&account_struct.f1_r_range_s,
							&account_struct.cubic_range_0_s,
							&account_struct.cubic_range_1_s,
							&mut account_struct.y0_range_s,
						);
					} else {
						//instruction 18 ---------------------------------------------
						instruction_order.push(18);
						assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
						assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
						assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
						assert_eq!(account_struct1.y6_range, account_struct.y6_range);
						_process_instruction(&mut account_struct1, 18);
						mul_assign_1_2(
							&account_struct.y0_range_s,
							&account_struct.i_range_s,
							&mut account_struct.cubic_range_0_s,
							&mut account_struct.cubic_range_1_s,
						);

						//instruction 19 ---------------------------------------------
						instruction_order.push(19);
						assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
						assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
						assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
						assert_eq!(account_struct1.y6_range, account_struct.y6_range);
						_process_instruction(&mut account_struct1, 19);
						mul_assign_3_4_5(
							&account_struct.i_range_s,
							&account_struct.cubic_range_0_s,
							&account_struct.cubic_range_1_s,
							&mut account_struct.y0_range_s,
						);
					}
				}
			}

			//instruction 20 ---------------------------------------------
			instruction_order.push(20);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 20);

			conjugate_wrapper(&mut account_struct.y0_range_s);
			assert_eq!(
				y0,
				parse_f_from_bytes(&account_struct.y0_range_s),
				"exp_by_neg_x(r) "
			);

			/*
				*
				*
				* ------------------------- y0.cyclotomic_square() -------------------------
				* still instruction 20
				*
				*/

			let y1 = y0.cyclotomic_square();
			custom_cyclotomic_square(&account_struct.y0_range_s, &mut account_struct.y1_range_s);
			assert_eq!(
				y1,
				parse_f_from_bytes(&account_struct.y1_range_s),
				"exp_by_neg_x(r) "
			);
			//y0 last used

			/*
				*
				*
				* ------------------------- y0.cyclotomic_square() -------------------------
				*
				*
				*/

			let y2 = y1.cyclotomic_square();
			//y2 is stored in y0_range_s
			//instruction 21 ---------------------------------------------
			instruction_order.push(21);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 21);

			custom_cyclotomic_square(&account_struct.y1_range_s, &mut account_struct.y0_range_s);
			assert_eq!(
				y2,
				parse_f_from_bytes(&account_struct.y0_range_s),
				"exp_by_neg_x(r) "
			);

			/*
				*
				*
				* ------------------------- mulassign -------------------------
				*
				*
				*/

			let mut y3 = y2 * &y1;
			//y3 is stored in y0_range_s

			//instruction 22 ---------------------------------------------
			instruction_order.push(22);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 22);
			mul_assign_1_2(
				&account_struct.y0_range_s,
				&account_struct.y1_range_s,
				&mut account_struct.cubic_range_0_s,
				&mut account_struct.cubic_range_1_s,
			);

			//instruction 23 ---------------------------------------------
			instruction_order.push(23);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 23);
			mul_assign_3_4_5(
				&account_struct.y1_range_s,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s,
				&mut account_struct.y0_range_s,
			);

			assert_eq!(
				y3,
				parse_f_from_bytes(&account_struct.y0_range_s),
				"mulassign "
			);

			/*
				*
				*
				* ------------------------- y4 = exp_by_neg_x(y3) -------------------------
				*
				*
				*/

			let y4 = exp_by_neg_x(y3);
			//y4 is stored in y2_range_s

			//init
			//instruction 24 ---------------------------------------------
			instruction_order.push(24);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 24);
			account_struct.i_range_s = account_struct.y0_range_s.clone();
			conjugate_wrapper(&mut account_struct.i_range_s);
			account_struct.y2_range_s = account_struct.y0_range_s.clone();

			for i in 1..63 {
				//cyclotomic_exp
				//instruction 25 ---------------------------------------------
				instruction_order.push(25);
				assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
				assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
				assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
				assert_eq!(account_struct1.y6_range, account_struct.y6_range);
				_process_instruction(&mut account_struct1, 25);
				custom_cyclotomic_square_in_place(&mut account_struct.y2_range_s);

				if NAF_VEC[i] != 0 {
					if NAF_VEC[i] > 0 {
						//instruction 26 ---------------------------------------------
						instruction_order.push(26);
						assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
						assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
						assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
						assert_eq!(account_struct1.y6_range, account_struct.y6_range);
						_process_instruction(&mut account_struct1, 26);
						mul_assign_1_2(
							&account_struct.y2_range_s,
							&account_struct.y0_range_s,
							&mut account_struct.cubic_range_0_s,
							&mut account_struct.cubic_range_1_s,
						);

						//instruction 27 ---------------------------------------------
						instruction_order.push(27);
						assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
						assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
						assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
						assert_eq!(account_struct1.y6_range, account_struct.y6_range);
						_process_instruction(&mut account_struct1, 27);
						mul_assign_3_4_5(
							&account_struct.y0_range_s,
							&account_struct.cubic_range_0_s,
							&account_struct.cubic_range_1_s,
							&mut account_struct.y2_range_s,
						);
					} else {
						//instruction 28 ---------------------------------------------
						instruction_order.push(28);
						assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
						assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
						assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
						assert_eq!(account_struct1.y6_range, account_struct.y6_range);
						_process_instruction(&mut account_struct1, 28);
						mul_assign_1_2(
							&account_struct.y2_range_s,
							&account_struct.i_range_s,
							&mut account_struct.cubic_range_0_s,
							&mut account_struct.cubic_range_1_s,
						);

						//instruction 29 ---------------------------------------------
						instruction_order.push(29);
						assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
						assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
						assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
						assert_eq!(account_struct1.y6_range, account_struct.y6_range);
						_process_instruction(&mut account_struct1, 29);
						mul_assign_3_4_5(
							&account_struct.i_range_s,
							&account_struct.cubic_range_0_s,
							&account_struct.cubic_range_1_s,
							&mut account_struct.y2_range_s,
						);
					}
				}
			}

			//instruction 30 ---------------------------------------------
			instruction_order.push(30);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 30);
			conjugate_wrapper(&mut account_struct.y2_range_s);

			assert_eq!(
				y4,
				parse_f_from_bytes(&account_struct.y2_range_s),
				"exp_by_neg_x(r) "
			);

			/*
				*
				*
				* ------------------------- y4.cyclotomic_square() -------------------------
				*
				* r_range: 			r
				* f_f2_range_s: 	f2 not used anymore
				* y0_range:  		y3
				* y1_range:			y1
				* y2_range:			y4
				* still instruction 30
				*/

			let y5 = y4.cyclotomic_square();
			//y5 is stored in f_f2_range_s
			custom_cyclotomic_square(&account_struct.y2_range_s, &mut account_struct.f_f2_range_s);
			assert_eq!(
				y5,
				parse_f_from_bytes(&account_struct.f_f2_range_s),
				"cyclotomic_square "
			);

			/*
				*
				*
				* ------------------------- y4 = exp_by_neg_x(y3) -------------------------
				*
				* r_range: 			r
				* f_f2_range_s: 	y5 			//y5 last used here
				* y0_range:  		y3
				* y1_range:			y1
				* y2_range:			y4
				* y6_range:			free
				*/

			let mut y6 = exp_by_neg_x(y5);
			//y4 is stored in y6_range

			//init
			//instruction 31 ---------------------------------------------
			instruction_order.push(31);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 31);
			account_struct.i_range_s = account_struct.f_f2_range_s.clone();

			conjugate_wrapper(&mut account_struct.i_range_s);

			account_struct.y6_range = account_struct.f_f2_range_s.clone();

			for i in 1..63 {
				//cyclotomic_exp
				//instruction 32 ---------------------------------------------
				instruction_order.push(32);
				assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
				assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
				assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
				assert_eq!(account_struct1.y6_range, account_struct.y6_range);
				_process_instruction(&mut account_struct1, 32);
				custom_cyclotomic_square_in_place(&mut account_struct.y6_range);

				if NAF_VEC[i] != 0 {
					if NAF_VEC[i] > 0 {
						//instruction 33 ---------------------------------------------
						instruction_order.push(33);
						assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
						assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
						assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
						assert_eq!(account_struct1.y6_range, account_struct.y6_range);
						_process_instruction(&mut account_struct1, 33);
						mul_assign_1_2(
							&account_struct.y6_range,
							&account_struct.f_f2_range_s,
							&mut account_struct.cubic_range_0_s,
							&mut account_struct.cubic_range_1_s,
						);

						//instruction 34 ---------------------------------------------
						instruction_order.push(34);
						assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
						assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
						assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
						assert_eq!(account_struct1.y6_range, account_struct.y6_range);
						_process_instruction(&mut account_struct1, 34);
						mul_assign_3_4_5(
							&account_struct.f_f2_range_s,
							&account_struct.cubic_range_0_s,
							&account_struct.cubic_range_1_s,
							&mut account_struct.y6_range,
						);
					} else {
						//instruction 35 ---------------------------------------------
						instruction_order.push(35);
						assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
						assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
						assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
						assert_eq!(account_struct1.y6_range, account_struct.y6_range);
						_process_instruction(&mut account_struct1, 35);
						mul_assign_1_2(
							&account_struct.y6_range,
							&account_struct.i_range_s,
							&mut account_struct.cubic_range_0_s,
							&mut account_struct.cubic_range_1_s,
						);

						//instruction 36 ---------------------------------------------
						instruction_order.push(36);
						assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
						assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
						assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
						assert_eq!(account_struct1.y6_range, account_struct.y6_range);
						_process_instruction(&mut account_struct1, 36);
						mul_assign_3_4_5(
							&account_struct.i_range_s,
							&account_struct.cubic_range_0_s,
							&account_struct.cubic_range_1_s,
							&mut account_struct.y6_range,
						);
					}
				}
			}
			//instruction 37 ---------------------------------------------
			instruction_order.push(37);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 37);

			//y6.conjugate();

			conjugate_wrapper(&mut account_struct.y6_range);

			assert_eq!(
				y6,
				parse_f_from_bytes(&account_struct.y6_range),
				"exp_by_neg_x(r) "
			);

			/*
				*
				*
				* ------------------------- conjugate_wrapper -------------------------
				*
				* r_range: 			r
				* f_f2_range_s: 	free
				* y0_range:  		y3
				* y1_range:			y1
				* y2_range:			y4
				* y6_range:			y6
				* instruction 37
				*/

			y3.conjugate();
			conjugate_wrapper(&mut account_struct.y0_range_s);

			y6.conjugate();

			conjugate_wrapper(&mut account_struct.y6_range);

			/*
				*
				*
				* ------------------------- mul_assign -------------------------
				*
				* r_range: 			r
				* f_f2_range_s: 	free
				* y0_range:  		y3
				* y1_range:			y1
				* y2_range:			y4
				* y6_range:			y6 last used
				*/

			let y7 = y6 * &y4;
			// stored in y6_range

			//instruction 38 ---------------------------------------------
			instruction_order.push(38);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 38);
			mul_assign_1_2(
				&account_struct.y6_range,
				&account_struct.y2_range_s,
				&mut account_struct.cubic_range_0_s,
				&mut account_struct.cubic_range_1_s,
			);

			//instruction 39 ---------------------------------------------
			instruction_order.push(39);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 39);
			mul_assign_3_4_5(
				&account_struct.y2_range_s,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s,
				&mut account_struct.y6_range,
			);

			assert_eq!(
				y7,
				parse_f_from_bytes(&account_struct.y6_range),
				"mulassign "
			);

			/*
				*
				*
				* ------------------------- mul_assign -------------------------
				*
				* r_range: 			r
				* f_f2_range_s: 	free
				* y0_range:  		y3 last used
				* y1_range:			y1
				* y2_range:			y4
				* y6_range:			y7 last used
				*/
			let mut y8 = y7 * &y3;
			// stored in y6_range

			//instruction 40 ---------------------------------------------
			instruction_order.push(40);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 40);
			mul_assign_1_2(
				&account_struct.y6_range,
				&account_struct.y0_range_s,
				&mut account_struct.cubic_range_0_s,
				&mut account_struct.cubic_range_1_s,
			);

			//instruction 41 ---------------------------------------------
			instruction_order.push(41);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 41);
			mul_assign_3_4_5(
				&account_struct.y0_range_s,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s,
				&mut account_struct.y6_range,
			);

			assert_eq!(
				y8,
				parse_f_from_bytes(&account_struct.y6_range),
				"mulassign "
			);

			/*
				*
				*
				* ------------------------- mul_assign -------------------------
				*
				* r_range: 			r
				* f_f2_range_s: 	free
				* y0_range:  		free
				* y1_range:			y1		last used
				* y2_range:			y4
				* y6_range:			y8
				*/
			let y9 = y8 * &y1;
			// stored in y1_range

			//instruction 42 ---------------------------------------------
			instruction_order.push(42);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 42);
			mul_assign_1_2(
				&account_struct.y1_range_s,
				&account_struct.y6_range,
				&mut account_struct.cubic_range_0_s,
				&mut account_struct.cubic_range_1_s,
			);

			//instruction 43 ---------------------------------------------
			instruction_order.push(43);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 43);
			mul_assign_3_4_5(
				&account_struct.y6_range,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s,
				&mut account_struct.y1_range_s,
			);

			assert_eq!(
				y9,
				parse_f_from_bytes(&account_struct.y1_range_s),
				"mulassign "
			);

			/*
				*
				*
				* ------------------------- mul_assign -------------------------
				*
				* r_range: 			r
				* f_f2_range_s: 	free
				* y0_range:  		free
				* y1_range:			y9
				* y2_range:			y4 last used
				* y6_range:			y8
				*/
			let y10 = y8 * &y4;
			// stored in y2_range_s

			//instruction 44 ---------------------------------------------
			instruction_order.push(44);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 44);
			mul_assign_1_2(
				&account_struct.y2_range_s,
				&account_struct.y6_range,
				&mut account_struct.cubic_range_0_s,
				&mut account_struct.cubic_range_1_s,
			);

			//instruction 45 ---------------------------------------------
			instruction_order.push(45);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 45);
			mul_assign_3_4_5(
				&account_struct.y6_range,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s,
				&mut account_struct.y2_range_s,
			);

			assert_eq!(
				y10,
				parse_f_from_bytes(&account_struct.y2_range_s),
				"mulassign "
			);

			/*
				*
				*
				* ------------------------- mul_assign -------------------------
				*
				* r_range: 			r
				* f_f2_range_s: 	free
				* y0_range:  		free
				* y1_range:			y9
				* y2_range:			y10  last used
				* y6_range:			y8
				*/
			let y11 = y10 * &r;
			// stored in y2_range_s

			//instruction 46 ---------------------------------------------
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 46);
			instruction_order.push(46);
			mul_assign_1_2(
				&account_struct.y2_range_s,
				&account_struct.f1_r_range_s,
				&mut account_struct.cubic_range_0_s,
				&mut account_struct.cubic_range_1_s,
			);

			//instruction 47 ---------------------------------------------
			instruction_order.push(47);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 47);
			mul_assign_3_4_5(
				&account_struct.f1_r_range_s,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s,
				&mut account_struct.y2_range_s,
			);

			assert_eq!(
				y11,
				parse_f_from_bytes(&account_struct.y2_range_s),
				"mulassign "
			);

			/*
				*
				*
				* ------------------------- assign -------------------------
				*
				* r_range: 			r
				* f_f2_range_s: 	free
				* y0_range:  		free
				* y1_range:			y9
				* y2_range:			y11
				* y6_range:			y8
				*/

			let mut y12 = y9;

			//instruction 48 ---------------------------------------------
			instruction_order.push(48);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 48);

			//assert_eq!(y12,  parse_f_from_bytes(&account_struct.y0_range_s));
			account_struct.y0_range_s = account_struct.y1_range_s.clone();
			/*
				*
				*
				* ------------------------- frobenius_map(1) -------------------------
				*
				* r_range: 			r
				* f_f2_range_s: 	free
				* y0_range:  		y12
				* y1_range:			y9
				* y2_range:			y11
				* y6_range:			y8
				*/
			y12.frobenius_map(1);

			custom_frobenius_map_1(&mut account_struct.y0_range_s);

			assert_eq!(y12, parse_f_from_bytes(&account_struct.y0_range_s));

			/*
				*
				*
				* ------------------------- mul_assign -------------------------
				*
				* r_range: 			r
				* f_f2_range_s: 	free
				* y0_range:  		y12
				* y1_range:			y9
				* y2_range:			y11 last used
				* y6_range:			y8
				*/
			let y13 = y12 * &y11;
			//y13 stored in y2_range_s

			//instruction 49 ---------------------------------------------
			instruction_order.push(49);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 49);
			mul_assign_1_2(
				&account_struct.y2_range_s,
				&account_struct.y0_range_s,
				&mut account_struct.cubic_range_0_s,
				&mut account_struct.cubic_range_1_s,
			);

			//instruction 50 ---------------------------------------------
			instruction_order.push(50);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 50);
			mul_assign_3_4_5(
				&account_struct.y0_range_s,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s,
				&mut account_struct.y2_range_s,
			);

			assert_eq!(
				y13,
				parse_f_from_bytes(&account_struct.y2_range_s),
				"mulassign "
			);

			/*
				*
				*
				* ------------------------- frobenius_map(2) -------------------------
				*
				* r_range: 			r
				* f_f2_range_s: 	free
				* y0_range:  		y12
				* y1_range:			y9
				* y2_range:			y13
				* y6_range:			y8
				*/
			y8.frobenius_map(2);

			//instruction 51 ---------------------------------------------
			instruction_order.push(51);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 51);
			custom_frobenius_map_2(&mut account_struct.y6_range);

			assert_eq!(y8, parse_f_from_bytes(&account_struct.y6_range));

			/*
				*
				*
				* ------------------------- mulassign -------------------------
				*
				* r_range: 			r
				* f_f2_range_s: 	free
				* y0_range:  		y12
				* y1_range:			y9
				* y2_range:			y13
				* y6_range:			y8 last used
				*/

			let y14 = y8 * &y13;
			//y14 stored in y6_range
			//instr110uctions already exist

			//instruction 38 ---------------------------------------------
			instruction_order.push(38);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 38);
			mul_assign_1_2(
				&account_struct.y6_range,
				&account_struct.y2_range_s,
				&mut account_struct.cubic_range_0_s,
				&mut account_struct.cubic_range_1_s,
			);

			//instruction 39 ---------------------------------------------
			instruction_order.push(39);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 39);
			mul_assign_3_4_5(
				&account_struct.y2_range_s,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s,
				&mut account_struct.y6_range,
			);

			assert_eq!(
				y14,
				parse_f_from_bytes(&account_struct.y6_range),
				"mulassign "
			);

			/*
				*
				*
				* ------------------------- conjugate -------------------------
				*
				* r_range: 			r
				* f_f2_range_s: 	free
				* y0_range:  		y12
				* y1_range:			y9
				* y2_range:			y13
				* y6_range:			y14
				*/

			//instruction 52 ---------------------------------------------
			instruction_order.push(52);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 52);
			r.conjugate();
			conjugate_wrapper(&mut account_struct.f1_r_range_s);

			/*
				*
				*
				* ------------------------- mul_assign -------------------------
				*
				* r_range: 			r		last used
				* f_f2_range_s: 	free
				* y0_range:  		y12
				* y1_range:			y9		last used
				* y2_range:			y13
				* y6_range:			y14
				*/

			let mut y15 = r * &y9;
			//y15 stored in y1_range

			//instruction 53 ---------------------------------------------
			instruction_order.push(53);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 53);
			mul_assign_1_2(
				&account_struct.y1_range_s,
				&account_struct.f1_r_range_s,
				&mut account_struct.cubic_range_0_s,
				&mut account_struct.cubic_range_1_s,
			);

			//instruction 54 ---------------------------------------------
			instruction_order.push(54);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 54);
			mul_assign_3_4_5(
				&account_struct.f1_r_range_s,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s,
				&mut account_struct.y1_range_s,
			);

			assert_eq!(
				y15,
				parse_f_from_bytes(&account_struct.y1_range_s),
				"mulassign "
			);

			/*
				*
				*
				* ------------------------- frobenius_map(3) -------------------------
				*
				* r_range: 			r
				* f_f2_range_s: 	free
				* y0_range:  		y12
				* y1_range:			y15
				* y2_range:			y13
				* y6_range:			y14
				*/

			y15.frobenius_map(3);

			//instruction 55 ---------------------------------------------
			instruction_order.push(55);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 55);
			custom_frobenius_map_3(&mut account_struct.y1_range_s);

			assert_eq!(y15, parse_f_from_bytes(&account_struct.y1_range_s));

			/*
				*
				*
				* ------------------------- mulassign -------------------------
				*
				* r_range: 			r
				* f_f2_range_s: 	free
				* y0_range:  		y12
				* y1_range:			y15
				* y2_range:			y13
				* y6_range:			y14
				*/
			//not unique second time instruction 83

			let y16 = y15 * &y14;

			//instruction 42 ---------------------------------------------
			instruction_order.push(42);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 42);
			mul_assign_1_2(
				&account_struct.y1_range_s,
				&account_struct.y6_range,
				&mut account_struct.cubic_range_0_s,
				&mut account_struct.cubic_range_1_s,
			);

			//instruction 43 ---------------------------------------------
			instruction_order.push(43);
			assert_eq!(account_struct1.y0_range_s, account_struct.y0_range_s);
			assert_eq!(account_struct1.y1_range_s, account_struct.y1_range_s);
			assert_eq!(account_struct1.y2_range_s, account_struct.y2_range_s);
			assert_eq!(account_struct1.y6_range, account_struct.y6_range);
			_process_instruction(&mut account_struct1, 43);
			mul_assign_3_4_5(
				&account_struct.y6_range,
				&account_struct.cubic_range_0_s,
				&account_struct.cubic_range_1_s,
				&mut account_struct.y1_range_s,
			);

			assert_eq!(
				y16,
				parse_f_from_bytes(&account_struct.y1_range_s),
				"mulassign "
			);
			println!(
				"Let instruction order: [u8; {}] = {:?}",
				instruction_order.len(),
				instruction_order
			);
			y16
		})
	}

	pub const INSTRUCTION_ORDER_CONST: [u8; 371] = [
		0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 10, 11, 14, 15, 15, 15, 15, 16, 17, 15, 15,
		16, 17, 15, 15, 15, 18, 19, 15, 15, 16, 17, 15, 15, 16, 17, 15, 15, 18, 19, 15, 15, 15, 16,
		17, 15, 15, 16, 17, 15, 15, 18, 19, 15, 15, 18, 19, 15, 15, 18, 19, 15, 15, 16, 17, 15, 15,
		15, 15, 16, 17, 15, 15, 15, 16, 17, 15, 15, 16, 17, 15, 15, 16, 17, 15, 15, 18, 19, 15, 15,
		16, 17, 15, 15, 15, 16, 17, 15, 15, 15, 15, 15, 16, 17, 15, 15, 16, 17, 15, 15, 15, 15, 15,
		18, 19, 15, 15, 15, 15, 16, 17, 20, 21, 22, 23, 24, 25, 25, 25, 25, 26, 27, 25, 25, 26, 27,
		25, 25, 25, 28, 29, 25, 25, 26, 27, 25, 25, 26, 27, 25, 25, 28, 29, 25, 25, 25, 26, 27, 25,
		25, 26, 27, 25, 25, 28, 29, 25, 25, 28, 29, 25, 25, 28, 29, 25, 25, 26, 27, 25, 25, 25, 25,
		26, 27, 25, 25, 25, 26, 27, 25, 25, 26, 27, 25, 25, 26, 27, 25, 25, 28, 29, 25, 25, 26, 27,
		25, 25, 25, 26, 27, 25, 25, 25, 25, 25, 26, 27, 25, 25, 26, 27, 25, 25, 25, 25, 25, 28, 29,
		25, 25, 25, 25, 26, 27, 30, 31, 32, 32, 32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 32, 35, 36,
		32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 35, 36, 32, 32, 32, 33, 34, 32, 32, 33, 34, 32, 32,
		35, 36, 32, 32, 35, 36, 32, 32, 35, 36, 32, 32, 33, 34, 32, 32, 32, 32, 33, 34, 32, 32, 32,
		33, 34, 32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 35, 36, 32, 32, 33, 34, 32, 32, 32, 33, 34,
		32, 32, 32, 32, 32, 33, 34, 32, 32, 33, 34, 32, 32, 32, 32, 32, 35, 36, 32, 32, 32, 32, 33,
		34, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 38, 39, 52, 53, 54, 55, 42,
		43,
	];

	fn final_exponentiation_test_processor(
		f: &<ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk,
	) -> Option<<ark_ec::models::bn::Bn<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk> {
		let mut account_struct = FinalExpBytes::new();

		parse_f_to_bytes(*f, &mut account_struct.f_f2_range_s);
		account_struct.changed_variables[F_F2_RANGE_ITER] = true;
		let mut account_onchain_slice = [0u8; 3900];
		<FinalExpBytes as Pack>::pack_into_slice(&account_struct, &mut account_onchain_slice);
		let path = "tests/fe_onchain_init_bytes.rs";
		let mut output = File::create(path).ok()?;
		write!(
			output,
			"{}",
			format!(
				"pub const INIT_BYTES_FINAL_EXP : [u8;{}] = {:?};",
				account_onchain_slice.len(),
				account_onchain_slice
			)
		);

		for i in INSTRUCTION_ORDER_CONST {
			let mut account_struct_tmp =
				<FinalExpBytes as Pack>::unpack(&account_onchain_slice).unwrap();
			println!("processor iter : {}", i);
			_process_instruction(&mut account_struct_tmp, i);
			account_struct.y1_range_s = account_struct_tmp.y1_range_s.clone();

			<FinalExpBytes as Pack>::pack_into_slice(
				&account_struct_tmp,
				&mut account_onchain_slice,
			);
			assert_eq!(account_struct.y1_range_s, account_struct_tmp.y1_range_s);
		}
		println!("result in bytes: {:?}", account_struct.y1_range_s);
		verify_result(&account_struct);
		Some(parse_f_from_bytes(&account_struct.y1_range_s))
	}
	use ark_ec::bn::BnParameters;
	use ark_ff::Fp12;
	pub fn exp_by_neg_x(
		mut f: Fp12<<ark_bn254::Parameters as ark_ec::bn::BnParameters>::Fp12Params>,
	) -> Fp12<<ark_bn254::Parameters as ark_ec::bn::BnParameters>::Fp12Params> {
		f = f.cyclotomic_exp(&<ark_bn254::Parameters as ark_ec::bn::BnParameters>::X);
		if !<ark_bn254::Parameters as ark_ec::bn::BnParameters>::X_IS_NEGATIVE {
			println!("conjugate");
			f.conjugate();
		}
		f
	}
}
