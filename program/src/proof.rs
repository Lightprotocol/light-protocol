use ark_bls12_381::*;
use ark_ec;
use ark_ff::biginteger::{BigInteger256, BigInteger384};
use ark_ff::fields::models::fp2::*;
use ark_ff::fields::models::quadratic_extension::QuadExtField;
use ark_ff::fields::models::quadratic_extension::QuadExtParameters;
use ark_ff::CubicExtField;
use ark_ff::{Fp256, Fp384};

pub fn get_proof_a() -> ark_ec::models::bls12::g1::G1Affine<ark_bls12_381::Parameters> {
    ark_ec::models::bls12::g1::G1Affine::<ark_bls12_381::Parameters>::new(
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([
            9860722045683943763,
            17310989719702591804,
            8654316632880884267,
            2565453678604647383,
            5527142138814667288,
            1327696982692448195,
        ])),
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([
            8598468961976368838,
            261670647750558011,
            18075667487803019378,
            8923310851665931633,
            18058147584816294847,
            1498014965056560393,
        ])),
        false,
    )
}

pub fn get_proof_b() -> ark_ec::models::bls12::g2::G2Affine<ark_bls12_381::Parameters> {
    ark_ec::models::bls12::g2::G2Affine::<ark_bls12_381::Parameters>::new(
        QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bls12_381::Fq2Parameters>>::new(
            Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([
                1089322373118118261,
                2169705576792371689,
                4436531080256321071,
                16749340808057792188,
                10198103467751861745,
                180412099324971246,
            ])),
            Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([
                1274367422994261851,
                9168134357110698602,
                277502271577622876,
                4036173852192167848,
                18235456077317286308,
                57920159041482304,
            ])),
        ),
        QuadExtField::<ark_ff::Fp2ParamsWrapper<ark_bls12_381::Fq2Parameters>>::new(
            Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([
                1912990397216453892,
                13316868795571702524,
                11689698958005661512,
                10874509049891812822,
                15840959066100489236,
                1833439678207659109,
            ])),
            Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([
                15370911824884984622,
                15493451345291507282,
                5703126377731634672,
                9652374120650843118,
                1917431421527100265,
                189722305847530821,
            ])),
        ),
        false,
    )
}
pub fn get_proof_c() -> ark_ec::models::bls12::g1::G1Affine<ark_bls12_381::Parameters> {
    ark_ec::models::bls12::g1::G1Affine::<ark_bls12_381::Parameters>::new(
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([
            16538290222162724967,
            6925146277010560159,
            12005764763774439926,
            1462071243163778953,
            14757842410725321191,
            1773261140342808754,
        ])),
        Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([
            12627750281439317552,
            13707270304729521898,
            10924654763188783400,
            7533259970498635595,
            1189007685602747742,
            1341303776336444445,
        ])),
        false,
    )
}
/*
pub  fn get_proof_hardcoded() -> Proof::<ark_ec::models::bls12::Bls12::<ark_bls12_381::Parameters>> {
    Proof::<ark_ec::models::bls12::Bls12::<ark_bls12_381::Parameters>> {
    a:
    ark_ec::models::bls12::g1::G1Affine::<ark_bls12_381::Parameters>::new(
    Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([9860722045683943763,
    17310989719702591804,
    8654316632880884267,
    2565453678604647383,
    5527142138814667288,
    1327696982692448195])),
    Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([8598468961976368838,
    261670647750558011,
    18075667487803019378,
    8923310851665931633,
    18058147584816294847,
    1498014965056560393])),

    false
    ),
    b:
    ark_ec::models::bls12::g2::G2Affine::<ark_bls12_381::Parameters>::new(
    QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
    Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([1089322373118118261,
    2169705576792371689,
    4436531080256321071,
    16749340808057792188,
    10198103467751861745,
    180412099324971246])),
    Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([1274367422994261851,
    9168134357110698602,
    277502271577622876,
    4036173852192167848,
    18235456077317286308,
    57920159041482304]))
    ),
    QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bls12_381::Fq2Parameters>>::new(
    Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([1912990397216453892,
    13316868795571702524,
    11689698958005661512,
    10874509049891812822,
    15840959066100489236,
    1833439678207659109])),
    Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([15370911824884984622,
    15493451345291507282,
    5703126377731634672,
    9652374120650843118,
    1917431421527100265,
    189722305847530821]))
    ),

    false
    ),
    c:
    ark_ec::models::bls12::g1::G1Affine::<ark_bls12_381::Parameters>::new(
    Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([16538290222162724967,
    6925146277010560159,
    12005764763774439926,
    1462071243163778953,
    14757842410725321191,
    1773261140342808754])),
    Fp384::<ark_bls12_381::FqParameters>::new(BigInteger384::new([12627750281439317552,
    13707270304729521898,
    10924654763188783400,
    7533259970498635595,
    1189007685602747742,
    1341303776336444445])),

    false
    )
    }
}*/
