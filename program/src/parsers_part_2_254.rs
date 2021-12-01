use ark_ff::bytes::{ToBytes, FromBytes};
use ark_ff::Fp256;
use ark_ec;
use ark_bn254;
use ark_ff::fields::models::quadratic_extension::{QuadExtField};
use num_traits::{One};
use solana_program::{
    msg,
    log::sol_log_compute_units,
};


pub fn parse_f_to_bytes_tmp(f : <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk, account: &mut [u8], range: [usize;2]){
    //
    let mut iter = range[0];
    for i in 0..2 as u8 {
        for j in 0..3 as u8 {
            for z in 0..2 as u8 {
                let tmp = iter;
                iter += 32;
                if i == 0 {
                    if j == 0 && z == 0 {
                        //println!("Parsing {:?}", f.c0.c0.c0);
                        <ark_bn254::Fq as ToBytes>::write(&f.c0.c0.c0, &mut account[tmp..iter]);
                    } else if j == 1 && z == 0 {
                        <ark_bn254::Fq as ToBytes>::write(&f.c0.c1.c0, &mut account[tmp..iter]);
                    } else if j == 2 && z == 0 {
                        <ark_bn254::Fq as ToBytes>::write(&f.c0.c2.c0, &mut account[tmp..iter]);
                    } else if j == 0 && z == 1 {
                        //println!("Parsing {:?}", f.c0.c0.c0);
                        <ark_bn254::Fq as ToBytes>::write(&f.c0.c0.c1, &mut account[tmp..iter]);
                    } else if j == 1 && z == 1 {
                        <ark_bn254::Fq as ToBytes>::write(&f.c0.c1.c1, &mut account[tmp..iter]);
                    } else if j == 2 && z == 1 {
                        <ark_bn254::Fq as ToBytes>::write(&f.c0.c2.c1, &mut account[tmp..iter]);
                    }
                } else if i == 1 {
                    if j == 0 && z == 0 {
                        //println!("Parsing {:?}", f.c0.c0.c0);
                        <ark_bn254::Fq as ToBytes>::write(&f.c1.c0.c0, &mut account[tmp..iter]);
                    } else if j == 1 && z == 0 {
                        <ark_bn254::Fq as ToBytes>::write(&f.c1.c1.c0, &mut account[tmp..iter]);
                    } else if j == 2 && z == 0 {
                        <ark_bn254::Fq as ToBytes>::write(&f.c1.c2.c0, &mut account[tmp..iter]);
                    } else if j == 0 && z == 1 {
                        //println!("Parsing {:?}", f.c0.c0.c0);
                        <ark_bn254::Fq as ToBytes>::write(&f.c1.c0.c1, &mut account[tmp..iter]);
                    } else if j == 1 && z == 1 {
                        <ark_bn254::Fq as ToBytes>::write(&f.c1.c1.c1, &mut account[tmp..iter]);
                    } else if j == 2 && z == 1 {
                        <ark_bn254::Fq as ToBytes>::write(&f.c1.c2.c1, &mut account[tmp..iter]);
                    }
                }
                }
            }
        }
}


pub fn parse_f_to_bytes_new(f : <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk, account: &mut Vec<u8>){
    //
    let mut iter = 0;
    for i in 0..2 as u8 {
        for j in 0..3 as u8 {
            for z in 0..2 as u8 {
                let tmp = iter;
                iter += 32;
                if i == 0 {
                    if j == 0 && z == 0 {
                        //println!("Parsing {:?}", f.c0.c0.c0);
                        <ark_bn254::Fq as ToBytes>::write(&f.c0.c0.c0, &mut account[tmp..iter]);
                    } else if j == 1 && z == 0 {
                        <ark_bn254::Fq as ToBytes>::write(&f.c0.c1.c0, &mut account[tmp..iter]);
                    } else if j == 2 && z == 0 {
                        <ark_bn254::Fq as ToBytes>::write(&f.c0.c2.c0, &mut account[tmp..iter]);
                    } else if j == 0 && z == 1 {
                        //println!("Parsing {:?}", f.c0.c0.c0);
                        <ark_bn254::Fq as ToBytes>::write(&f.c0.c0.c1, &mut account[tmp..iter]);
                    } else if j == 1 && z == 1 {
                        <ark_bn254::Fq as ToBytes>::write(&f.c0.c1.c1, &mut account[tmp..iter]);
                    } else if j == 2 && z == 1 {
                        <ark_bn254::Fq as ToBytes>::write(&f.c0.c2.c1, &mut account[tmp..iter]);
                    }
                } else if i == 1 {
                    if j == 0 && z == 0 {
                        //println!("Parsing {:?}", f.c0.c0.c0);
                        <ark_bn254::Fq as ToBytes>::write(&f.c1.c0.c0, &mut account[tmp..iter]);
                    } else if j == 1 && z == 0 {
                        <ark_bn254::Fq as ToBytes>::write(&f.c1.c1.c0, &mut account[tmp..iter]);
                    } else if j == 2 && z == 0 {
                        <ark_bn254::Fq as ToBytes>::write(&f.c1.c2.c0, &mut account[tmp..iter]);
                    } else if j == 0 && z == 1 {
                        //println!("Parsing {:?}", f.c0.c0.c0);
                        <ark_bn254::Fq as ToBytes>::write(&f.c1.c0.c1, &mut account[tmp..iter]);
                    } else if j == 1 && z == 1 {
                        <ark_bn254::Fq as ToBytes>::write(&f.c1.c1.c1, &mut account[tmp..iter]);
                    } else if j == 2 && z == 1 {
                        <ark_bn254::Fq as ToBytes>::write(&f.c1.c2.c1, &mut account[tmp..iter]);
                    }
                }
                }
            }
        }
}

pub fn parse_f_from_bytes_new(account: &Vec<u8>) -> <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk {

    let mut iter = 0; // should be 0
    let mut f = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::one();

    for i in 0..2 as u8 {
        for j in 0..3 as u8 {
            for z in 0..2 as u8 {
                let tmp = iter;
                iter += 32;
                if i == 0 {
                    if j == 0 && z == 0 {
                        f.c0.c0.c0 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                    } else if j == 1 && z == 0 {
                        f.c0.c1.c0 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                    } else if j == 2 && z == 0 {
                        f.c0.c2.c0 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                    } else if j == 0 && z == 1 {
                        f.c0.c0.c1 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                    } else if j == 1 && z == 1 {
                        f.c0.c1.c1 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                    } else if j == 2 && z == 1 {
                        f.c0.c2.c1 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                    }
                } else if i == 1 {
                    if j == 0 && z == 0 {
                        f.c1.c0.c0 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                    } else if j == 1 && z == 0 {
                        f.c1.c1.c0 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                    } else if j == 2 && z == 0 {
                        f.c1.c2.c0 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                    } else if j == 0 && z == 1 {
                        f.c1.c0.c1 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                    } else if j == 1 && z == 1 {
                        f.c1.c1.c1 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                    } else if j == 2 && z == 1 {
                        f.c1.c2.c1 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                    }
                }
                }
            }
        }
    f
}



pub fn parse_f_from_bytes_tmp(account: &[u8], range: [usize;2]) -> <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk {

    let mut iter = range[0]; // should be 0
    let mut f = <ark_ec::models::bn::Bn::<ark_bn254::Parameters> as ark_ec::PairingEngine>::Fqk::one();

    for i in 0..2 as u8 {
        for j in 0..3 as u8 {
            for z in 0..2 as u8 {
                let tmp = iter;
                iter += 32;
                if i == 0 {
                    if j == 0 && z == 0 {
                        f.c0.c0.c0 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                    } else if j == 1 && z == 0 {
                        f.c0.c1.c0 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                    } else if j == 2 && z == 0 {
                        f.c0.c2.c0 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                    } else if j == 0 && z == 1 {
                        f.c0.c0.c1 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                    } else if j == 1 && z == 1 {
                        f.c0.c1.c1 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                    } else if j == 2 && z == 1 {
                        f.c0.c2.c1 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                    }
                } else if i == 1 {
                    if j == 0 && z == 0 {
                        f.c1.c0.c0 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                    } else if j == 1 && z == 0 {
                        f.c1.c1.c0 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                    } else if j == 2 && z == 0 {
                        f.c1.c2.c0 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                    } else if j == 0 && z == 1 {
                        f.c1.c0.c1 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                    } else if j == 1 && z == 1 {
                        f.c1.c1.c1 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                    } else if j == 2 && z == 1 {
                        f.c1.c2.c1 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                    }
                }
                }
            }
        }
    f
}


fn parse_fp256_to_bytes(fp256 : ark_bn254::Fq, account: &mut [u8], range: [usize;2]){

    let start = range[0];
    let end = range[1];
    <ark_bn254::Fq as ToBytes>::write(&fp256, &mut account[start..end]);
}

pub fn parse_fp256_to_bytes_new(fp256 : ark_bn254::Fq, account: &mut Vec<u8>){

    let start = 0;
    let end = 32;
    <ark_bn254::Fq as ToBytes>::write(&fp256, &mut account[start..end]);
}


fn parse_fp256_from_bytes(account: &[u8], range: [usize;2]) -> ark_bn254::Fq{
    let fp256: ark_bn254::Fq;
    let start = range[0];
    let end = range[1];
    fp256 = <ark_bn254::Fq as FromBytes>::read(&account[start..end]).unwrap();
    fp256
}

pub fn parse_fp256_from_bytes_new(account: &Vec<u8>) -> ark_bn254::Fq{
    let fp256: ark_bn254::Fq;
    let start = 0;
    let end = 32;
    fp256 = <ark_bn254::Fq as FromBytes>::read(&account[start..end]).unwrap();
    fp256
}


fn parse_quad_to_bytes(q : ark_ff::QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>, account: &mut [u8], range: [usize;2]){

    let mut iter = range[0];

        for z in 0..2 as u8 {
            let tmp = iter;
            iter += 32;
                if z == 0 {
                    //println!("Parsing {:?}", c.c0);
                    <ark_bn254::Fq as ToBytes>::write(&q.c0, &mut account[tmp..iter]);
                } else if z == 1 {
                    <ark_bn254::Fq as ToBytes>::write(&q.c1, &mut account[tmp..iter]);
                }
        }
}

pub fn parse_quad_to_bytes_new(q : ark_ff::QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>>, account: &mut Vec<u8>){

    let mut iter = 0;

        for z in 0..2 as u8 {
            let tmp = iter;
            iter += 32;
                if z == 0 {
                    //println!("Parsing {:?}", c.c0);
                    <ark_bn254::Fq as ToBytes>::write(&q.c0, &mut account[tmp..iter]);
                } else if z == 1 {
                    <ark_bn254::Fq as ToBytes>::write(&q.c1, &mut account[tmp..iter]);
                }
        }
}


fn parse_quad_from_bytes(account: &[u8], range:[usize;2]) -> ark_ff::QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>> {

    let start = range[0];
    let end = range[1];
    let iter = start + 32;

    let quad = QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
        <ark_bn254::Fq as FromBytes>::read(&account[start..iter]).unwrap(),
        <ark_bn254::Fq as FromBytes>::read(&account[iter..end]).unwrap()
    );
    quad

}


pub fn parse_quad_from_bytes_new(account: &Vec<u8>) -> ark_ff::QuadExtField<ark_ff::Fp2ParamsWrapper<ark_bn254::Fq2Parameters>> {

    let start = 0;
    let end = 64;
    let iter = start + 32;

    let quad = QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
        <ark_bn254::Fq as FromBytes>::read(&account[start..iter]).unwrap(),
        <ark_bn254::Fq as FromBytes>::read(&account[iter..end]).unwrap()
    );
    quad

}

fn parse_cubic_to_bytes(c : ark_ff::CubicExtField<ark_ff::Fp6ParamsWrapper<ark_bn254::Fq6Parameters>>, account: &mut [u8], range: [usize;2]) {

    let mut iter = range[0];
    for j in 0..3 as u8 {
        for z in 0..2 as u8 {
            let tmp = iter;
            iter += 32;
                if j == 0 && z == 0 {
                    //println!("Parsing {:?}", f.c0.c0.c0);
                    <ark_bn254::Fq as ToBytes>::write(&c.c0.c0, &mut account[tmp..iter]);
                } else if j == 1 && z == 0 {
                    <ark_bn254::Fq as ToBytes>::write(&c.c1.c0, &mut account[tmp..iter]);
                } else if j == 2 && z == 0 {
                    <ark_bn254::Fq as ToBytes>::write(&c.c2.c0, &mut account[tmp..iter]);
                } else if j == 0 && z == 1 {
                    //println!("Parsing {:?}", f.c0.c0.c0);
                    <ark_bn254::Fq as ToBytes>::write(&c.c0.c1, &mut account[tmp..iter]);
                } else if j == 1 && z == 1 {
                    <ark_bn254::Fq as ToBytes>::write(&c.c1.c1, &mut account[tmp..iter]);
                } else if j == 2 && z == 1 {
                    <ark_bn254::Fq as ToBytes>::write(&c.c2.c1, &mut account[tmp..iter]);
                }
        }
    }
}

pub fn parse_cubic_to_bytes_new(c : ark_ff::CubicExtField<ark_ff::Fp6ParamsWrapper<ark_bn254::Fq6Parameters>>, account: &mut Vec<u8>,range:[usize;2]) {

    let mut iter = range[0];
    for j in 0..3 as u8 {
        for z in 0..2 as u8 {
            let tmp = iter;
            iter += 32;
                if j == 0 && z == 0 {
                    //println!("Parsing {:?}", f.c0.c0.c0);
                    <ark_bn254::Fq as ToBytes>::write(&c.c0.c0, &mut account[tmp..iter]);
                } else if j == 1 && z == 0 {
                    <ark_bn254::Fq as ToBytes>::write(&c.c1.c0, &mut account[tmp..iter]);
                } else if j == 2 && z == 0 {
                    <ark_bn254::Fq as ToBytes>::write(&c.c2.c0, &mut account[tmp..iter]);
                } else if j == 0 && z == 1 {
                    //println!("Parsing {:?}", f.c0.c0.c0);
                    <ark_bn254::Fq as ToBytes>::write(&c.c0.c1, &mut account[tmp..iter]);
                } else if j == 1 && z == 1 {
                    <ark_bn254::Fq as ToBytes>::write(&c.c1.c1, &mut account[tmp..iter]);
                } else if j == 2 && z == 1 {
                    <ark_bn254::Fq as ToBytes>::write(&c.c2.c1, &mut account[tmp..iter]);
                }
        }
    }
}

fn parse_cubic_from_bytes(account: &[u8],range:[usize;2]) -> ark_ff::CubicExtField<ark_ff::Fp6ParamsWrapper<ark_bn254::Fq6Parameters>> {
    let mut iter = range[0]; // should be 0
    let mut cubic = ark_ff::CubicExtField::<ark_ff::Fp6ParamsWrapper::<ark_bn254::Fq6Parameters>>::one();
    for j in 0..3 as u8 {
        for z in 0..2 as u8 {
            let tmp = iter;
            iter += 32;
                if j == 0 && z == 0 {
                    cubic.c0.c0 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                } else if j == 1 && z == 0 {
                    cubic.c1.c0 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                } else if j == 2 && z == 0 {
                    cubic.c2.c0 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                } else if j == 0 && z == 1 {
                    cubic.c0.c1 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                } else if j == 1 && z == 1 {
                    cubic.c1.c1 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                } else if j == 2 && z == 1 {
                    cubic.c2.c1 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                }
            }
        }
        cubic
}

pub fn parse_cubic_from_bytes_new(account: &Vec<u8>,range:[usize;2]) -> ark_ff::CubicExtField<ark_ff::Fp6ParamsWrapper<ark_bn254::Fq6Parameters>> {
    let mut iter = range[0];
    let mut cubic = ark_ff::CubicExtField::<ark_ff::Fp6ParamsWrapper::<ark_bn254::Fq6Parameters>>::one();
    for j in 0..3 as u8 {
        for z in 0..2 as u8 {
            let tmp = iter;
            iter += 32;
                if j == 0 && z == 0 {
                    cubic.c0.c0 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                } else if j == 1 && z == 0 {
                    cubic.c1.c0 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                } else if j == 2 && z == 0 {
                    cubic.c2.c0 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                } else if j == 0 && z == 1 {
                    cubic.c0.c1 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                } else if j == 1 && z == 1 {
                    cubic.c1.c1 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                } else if j == 2 && z == 1 {
                    cubic.c2.c1 = <ark_bn254::Fq as FromBytes>::read(&account[tmp..iter]).unwrap();
                }
            }
        }
        cubic
}
