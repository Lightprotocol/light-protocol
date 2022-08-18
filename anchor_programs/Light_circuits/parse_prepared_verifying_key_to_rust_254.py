

def mode_reached_gamma_abc_g1(word, result_file, counter):
    if word == "x:":
        result_file.write("")
    elif word == "y:":
        result_file.write("")
    elif word == "GroupAffine" or word == "[GroupAffine":
        result_file.write("\n\n")
        result_file.write("pub fn get_gamma_abc_g1_"+ str(counter) +"() -> ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters> { \n \t")
        result_file.write("ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(")
        counter +=1
    elif word[0:5] == "Fp256":
        result_file.write("\n\t\tark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([" + word.split("[",1)[1])
    elif word == "{":
        result_file.write("")
    elif word == "infinity:":
        result_file.write("")
    elif word == "},":
        result_file.write("\n\t)")
        result_file.write("\n}")
        result_file.write("")
    elif word == "},":
        result_file.write("\n),")
    elif word == "}]":
        result_file.write("")
        #result_file.write(")")
    else:
        result_file.write(word + " ")
    return counter

def mode_reached_g1(word, result_file, counter, id):
    if word == "x:":
        result_file.write("")
    elif word == "y:":
        result_file.write("")
    elif word == "GroupAffine" or word == "[GroupAffine":
        result_file.write("\n\n")
        result_file.write("pub fn get_"+ id + "_"+ str(counter) +"() -> ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters> { \n \t")
        result_file.write("ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(")
        counter +=1
    elif word[0:5] == "Fp256":
        result_file.write("\n\t\tark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([" + word.split("[",1)[1])
    elif word == "{":
        result_file.write("")
    elif word == "infinity:":
        result_file.write("")
    elif word == "},":
        result_file.write("\n\t)")
        result_file.write("\n}")
        result_file.write("")
    elif word == "},":
        result_file.write("\n),")
    elif word == "}]":
        result_file.write("")
        #result_file.write(")")
    else:
        result_file.write(word + " ")
    return counter

def mode_reached_g2(word, result_file, counter, id):
    if word == "x:":
        result_file.write("")
    elif word == "y:":
        result_file.write("")
    elif word == "GroupAffine" or word == "[GroupAffine":
        result_file.write("\n\n")
        result_file.write("pub fn get_"+ id + "_"+ str(counter) +"() -> ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters> { \n \t")
        result_file.write("ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(")
        counter +=1
    elif word == "QuadExtField":
        result_file.write("\n\t\tQuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(")
    elif word[0:5] == "Fp256":
        result_file.write("\n\t\t\tark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([" + word.split("[",1)[1])
    elif word == "{":
        result_file.write("")
    elif word == "infinity:":
        result_file.write("")
    elif word == "},":
        result_file.write("\n\t\t),")
        #result_file.write("\n}")
        result_file.write("")
    elif word == "}":
        result_file.write("")
    elif word == "}]":
        result_file.write("")
    elif word == "c0:":
        result_file.write("")
    elif word == "c1:":
        result_file.write("")
    elif word == "false":
        result_file.write("\n\t\t" + word + "\n\t)")
        return -1
    else:
        result_file.write(word + " ")
    return counter


def mode_reached_g2_prepared(word, result_file, counter, id):
    if word == "x:":
        result_file.write("")
    elif word == "y:":
        result_file.write("")
    elif word == "(QuadExtField" or word == "[(QuadExtField":
        result_file.write("\n\n")
        result_file.write("pub fn get_"+ id + "_"+ str(counter) +"() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { \n \t")
        result_file.write("(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(")
        counter +=1
    elif word == "QuadExtField":
        result_file.write("\n\t\tQuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(")
    elif word[0:5] == "Fp256":
        result_file.write("\n\t\t\tark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([" + word.split("[",1)[1])
    elif word == "{":
        result_file.write("")
    elif word == "infinity:":
        result_file.write("")
    elif word == "},":
        result_file.write("\n\t\t),")
        #result_file.write("\n}")
        result_file.write("")
    elif word == "}":
        result_file.write("")
    elif word == "}),":
        result_file.write("\n \t\t)\n \t)\n}")
    elif word == "c0:":
        result_file.write("")
    elif word == "c1:":
        result_file.write("")
    elif word == "G2Prepared":
        result_file.write("")
    elif word == "ell_coeffs:":
        result_file.write("")
    elif word == "})],":
        result_file.write("\n \t\t)\n \t)")
    elif word == "false":
        #result_file.write("\n\t\t" + word + "\n\t)")
        return -1
    else:
        result_file.write(word + " ")
    return counter

l = []
f = open("prepared_verifying_key.txt", "r")
result_file = open('src/hard_coded_pvk_22_1.rs','w')
#result_file.write("use ark_ff::Fp256;\n")
result_file.write("use ark_ff::biginteger::BigInteger256;\n")
#result_file.write("use ark_ec::{models::twisted_edwards_extended::GroupAffine};\n")
result_file.write("use ark_ff::QuadExtField;\n")

#result_file.write("use ark_bn254::*;\n")
reached_alpha_g1 = False
reached_beta_g2 = False

reached_gamma_g2 = False
reached_delta_g2 = False

reached_gamma_abc_g1 = False

reached_gamma_g2_neg_pc = False
reached_delta_g2_neg_pc= False
reached_reached_ALPHA_G1_BETA_G2= False

counter = 0
for line in f:
    for word in line.split():
        #l.append(word)
        #result_file.write(word)
        if word == "alpha_g1:":
            reached_alpha_g1 = True
            counter = 0
        elif reached_alpha_g1:
            if word == "beta_g2:":
                reached_alpha_g1 = False
                reached_beta_g2 = True
                counter = 0
                continue

            counter = mode_reached_g1(word, result_file, counter, "alpha_g1")
        elif word == "beta_g2:":
            reached_beta_g2 = True
            counter = 0
        elif reached_beta_g2:
            if word == "gamma_g2:":
                reached_beta_g2 = False
                reached_gamma_g2 = True
                counter = 0
                continue
            elif counter == -1:
                result_file.write("\n}")
                continue
            counter = mode_reached_g2(word, result_file, counter, "beta_g2")
        elif word == "gamma_g2:":
            reached_gamma_g2 = True
            counter = 0
        elif reached_gamma_g2:
            if word == "delta_g2:":
                reached_gamma_g2 = False
                reached_delta_g2 = True
                counter = 0
                continue
            elif counter == -1:
                result_file.write("\n}")
                continue
            counter = mode_reached_g2(word, result_file, counter, "gamma_g2")
        elif word == "delta_g2:":
            reached_delta_g2 = True
            counter = 0
        elif reached_delta_g2:
            if word == "gamma_abc_g1:":
                reached_delta_g2 = False
                reached_gamma_abc_g1 = True
                counter = 0
                continue
            elif counter == -1:
                result_file.write("\n}")
                continue
            counter = mode_reached_g2(word, result_file, counter, "delta_g2")
        elif word == "gamma_abc_g1:":
            delta_g2 = True
        elif reached_gamma_abc_g1:
            if word == "alpha_g1_beta_g2:":
                reached_gamma_abc_g1 = False
                counter = 0
                continue
            counter = mode_reached_gamma_abc_g1(word, result_file, counter)
        elif word == "gamma_g2_neg_pc:":
            reached_gamma_g2_neg_pc = True
        elif reached_gamma_g2_neg_pc:
            if word == "delta_g2_neg_pc:":
                reached_gamma_g2_neg_pc = False
                reached_delta_g2_neg_pc = True
                counter = 0
                continue
            elif counter == -1:
                result_file.write("\n}")
                continue
            counter = mode_reached_g2_prepared(word, result_file, counter, "gamma_g2_neg_pc")
        elif word == "delta_g2_neg_pc:":
            reached_delta_g2_neg_pc = True
        elif reached_delta_g2_neg_pc:
            if word == "pub":
                reached_delta_g2_neg_pc = False
                counter = 0
                reached_reached_ALPHA_G1_BETA_G2 = True
                result_file.write("\n")
            elif counter == -1:
                result_file.write("\n}")
                reached_gamma_g2_neg_pc = False
                break
            counter = mode_reached_g2_prepared(word, result_file, counter, "delta_g2_neg_pc")
        elif reached_reached_ALPHA_G1_BETA_G2:
            result_file.write(word + " ")
        else:
            continue
f.close()
result_file.close()

#generate tests
test_file = open('src/tests_pvk.rs','w')

#test_file.write("#[test]\n")
id = "gamma_abc_g1"
for i in range(0,5):
    test_file.write("assert_eq!(get_"+ id + "_"+ str(i) +"(), pvk.vk." + id + "[" + str(i) + "]);\n")


id = "gamma_g2_neg_pc"
for i in range(0,68):
    test_file.write("assert_eq!(get_"+ id + "_"+ str(i) +"() , pvk." + id + ".ell_coeffs[" + str(i) + "]);\n")


id = "delta_g2_neg_pc"
for i in range(0,68):
    test_file.write("assert_eq!(get_"+ id + "_"+ str(i) +"() , pvk." + id + ".ell_coeffs[" + str(i) + "]);\n")
