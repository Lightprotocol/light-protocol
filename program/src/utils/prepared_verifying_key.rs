use ark_ff::biginteger::BigInteger256;
use ark_ff::QuadExtField;


pub fn get_alpha_g1_0() -> ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters> { 
 	ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10802560400167329898,4427831596641847289, 8446202072820133997, 125567519208212759])), 
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11751420931253826222,8710609348661928048, 4579119150976691833, 2516190483927819264])), false 
	)
}

pub fn get_beta_g2_0() -> ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters> { 
 	ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13957427368410267022,6101563348155213896, 15595705088219937741, 2977830004324324263])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7792285146718175302,14431480397106960593, 8891978072743389075, 172671505553152981])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([383450373217395126,3227181728795496973, 4495371084454688693, 522152305940961108])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18054170617781614761,18267147348858885998, 3733588098824404793, 644943871665481251])) 
		),
		false
	)
}

pub fn get_gamma_g2_0() -> ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters> { 
 	ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10269251484633538598,15918845024527909234, 18138289588161026783, 1825990028691918907])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12660871435976991040,6936631231174072516, 714191060563144582, 1512910971262892907])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7034053747528165878,18338607757778656120, 18419188534790028798, 2953656481336934918])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7208393106848765678,15877432936589245627, 6195041853444001910, 983087530859390082])) 
		),
		false
	)
}

pub fn get_delta_g2_0() -> ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters> { 
 	ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2230018614732996203,4816166062302509891, 6985138768017785972, 1965244750779376408])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10036116687961783745,6719835143815648439, 10578165907174441763, 1723822096419494877])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([258562452960205554,336930387013674631, 8585034722585991985, 3165887118114759475])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8166518649785466642,1983488906777032420, 14277287222950976866, 1966563496553132551])) 
		),
		false
	)
}

pub fn get_gamma_abc_g1_0() -> ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters> { 
 	ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1066727571854014131,12441028284001198887, 13384027390696184766, 2208197633686406652])), 
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9330012877489461198,17502195030746717405, 14803056417701887008, 322308098704544156])), false 
	)
}

pub fn get_gamma_abc_g1_1() -> ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters> { 
 	ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15096305088613038041,15450916936429664742, 7012240460884899246, 92513671536051841])), 
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10862194231830853264,11056503687343061069, 12290720852439755401, 459342416520978194])), false 
	)
}

pub fn get_gamma_abc_g1_2() -> ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters> { 
 	ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15641644508138297079,9041321002551705920, 15800807934332706761, 2656166353491503414])), 
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16268602035483486825,10972259957439130024, 9623762579438884381, 2012664647234000075])), false 
	)
}

pub fn get_gamma_abc_g1_3() -> ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters> { 
 	ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8692429926662127593,10935535300992320083, 9787298698866778316, 1975884577793139361])), 
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10120282717069195739,2924651772574208664, 16355418730909155077, 1971857561760981895])), false 
	)
}

pub fn get_gamma_abc_g1_4() -> ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters> { 
 	ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10144044385156409655,610303884894678897, 9223997014402146128, 509492180233071587])), 
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10996773709255984240,13514726858264744480, 4671917309208571268, 1887003877686392772])), false 
	)
}

pub fn get_gamma_abc_g1_5() -> ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters> { 
 	ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16437579820482701014,7600137282207214753, 15411185132147219912, 3201153759406335437])), 
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2017090504016175258,7162404748314854399, 11499298508078865969, 2757817966897549870])), false 
	)
}

pub fn get_gamma_abc_g1_6() -> ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters> { 
 	ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14057919031671944666,14203845255446805835, 9015258206618143795, 964845734187903972])), 
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5791413923656349836,17403608054330843338, 2071208950826848467, 1879590606605722516])), false 
	)
}

pub fn get_gamma_abc_g1_7() -> ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters> { 
 	ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1881165474423442217,4233378363008024081, 5658078745662843270, 3270044658850523312])), 
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8269266514865638031,15906990907728944814, 12488463804295023706, 2788653403667020252])), false 
	)
}

pub fn get_gamma_g2_neg_pc_0() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9735490623776675493,7313347297369877603, 5110441044595811232, 2420314695870899172])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14416786213697531356,13308121799468939638, 12390083706888003821, 1966175061718780164])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4129257347520110928,13798226624051452651, 4825670390762580777, 1989277302133421735])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11104699749248547751,10435997551076758402, 17853110753348405340, 3361471515497012039])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10169789301848189331,16016180067228186549, 17334750741304028879, 2228788662616803775])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15172957284714629703,13417154794643176123, 3196086454825695542, 2093866205601446741])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_1() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6373452946747570674,10270768430483208834, 11341147745087012459, 3157052191146643204])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([602066187160159699,1578931260951444474, 1587541677266892445, 1992373586887236310])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2035058501502628319,14930432017151590998, 15355551583521351086, 469587794589787657])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13631719073532767446,2065158137318837312, 5775538604822855962, 2383111915651801787])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17544310453790923341,14459545592572037104, 12200103993180316021, 2090533022732391846])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8054743094808658598,15345477660971473493, 15443796689430031587, 1806066076678295575])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_2() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1307490507467590467,15702387221270537235, 5269129970681753992, 28002378715318771])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4660357668607340467,16578489089999929478, 16748828149682735846, 739979399064110919])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4578782715327068294,8628960991187287885, 16091835164139194461, 852554802780718793])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12096117391014189539,18239444815601499298, 7233070439485440435, 3194275071475042713])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5827190618476342597,12470106628583867316, 17200718410803934706, 550783932675933241])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15647736919678071305,12053702395563955525, 4650097433064156528, 2812014987399368919])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_3() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1836789663054811950,11512141512792473873, 1478746144118729173, 2923691560477017483])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12829111010025739515,7250912810722826978, 8129015919716064956, 1557336640775108833])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16722324985467231953,3455514969581926786, 5435134192097375645, 445144570921449663])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17552173304110772017,6392449665810583181, 15317764502253575963, 1986700432257093656])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5236977799088141757,16542968308152612384, 11217115100283931318, 2197361333128902643])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10433619994713991449,5468729870700106286, 13731897016295146488, 3282210527109190724])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_4() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4097647836534978155,16211718821644978109, 12543988062359842685, 531950869723565272])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3732584527186327026,11009433307933628691, 9906125292583317817, 922464594331819969])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15175933070488744052,8557412162374306745, 1363276520257215091, 1206507072649020906])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1439564256118195292,11404518156846947668, 5178720706420544533, 1208701127683364254])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16144253490566453732,4504249177135800213, 367723188584878275, 2190328921017053644])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10025827719085050808,15989316882772203996, 15449519052851461310, 1313861631468371700])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_5() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5823280062939564169,15680090768088578823, 15930926657659411240, 447669662857831647])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7123031478800885477,11854536813934295290, 4576838324085926162, 393867877332217377])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([369312874939631957,5577468896030353349, 16411512831196144769, 2225425689593265692])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6638295224199888525,18378089365476925535, 9535882037841911296, 2499032369420772404])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13635119636550970561,6755198425354098277, 14942399890630288505, 1458437123855160184])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7719400999163966459,8978950345857190867, 3976775237719123842, 403317986652656893])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_6() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6323944148176042385,14827035312785847748, 7085342050920843499, 2194884312546864639])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10713727812974110426,15122847604125151928, 7969503129420113999, 3072497528924008776])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8245328154565749483,15008057688241463482, 1904435821669162144, 1441912631713626900])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14067806317296624525,5017805370971101456, 14326143508175705321, 2028047398688701706])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12042115942287690287,16050192987958347428, 1145228044111305845, 2627988669539177495])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13047123149285011562,15386613089455414049, 7510911058351255393, 3316332504285088137])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_7() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7891890699752675087,12715027985949561209, 6042813899840893100, 868694344373622319])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16399427437888050820,3846046456776588454, 10686023346950737987, 3344759956442768000])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12294815732816214463,3961530758447984123, 15628978538598733560, 3202928354188042095])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18252201952929349815,6216908994703533045, 14438125476595334964, 1047171371043863825])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13026834433791157246,14765348982607191910, 16505480142318392620, 1882850159514956635])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([333599781101283695,17810853688536321163, 16496026278973326277, 2847222041893320289])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_8() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7116649517358614829,2920647569665513542, 13209731436924262946, 453639253430949154])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3666803213437464831,7451829943988360517, 7980987991301795264, 2666344424132976136])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5219278092499055320,3884916115576163386, 16328782425056420861, 3301475912234288630])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3431571446564174036,11493009609057756909, 8618636858343857939, 1986863209228296802])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9074211749247733676,2418817438739424044, 679117380560856971, 169995907362283696])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10385737847901021227,14315582513327762173, 9738557493926035032, 950471749667491902])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_9() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5252025707559437409,13953637720164835353, 3419520033516939334, 120015468621981738])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11981700070783507295,1570822996567796465, 7857295077868291515, 2144628409971280383])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12620013865667236324,15851160004860545728, 15601915586305788059, 2284009028467699413])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12661019300699867038,3364324049985048219, 17977672049062988437, 1576103176157591547])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18429994302768323336,13813699275930670289, 11011949919505124260, 2042408909137613241])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16574176000712322217,10294407224739185804, 11693088177603906544, 639379204620249221])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_10() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11961844684395492461,15529371282921715378, 17772386125370065890, 959811724919934129])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1378846033346520785,7389322072149155598, 1592202723981715011, 2731071939212094142])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3639945603867169008,12646131179701256118, 5410141380190428528, 3332492611662908087])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([270412619520218290,10788974735074041043, 11640321094267608914, 2983084359777738873])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12862302995183110164,11021245783930228767, 8237907725727474589, 2863977377025820330])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3599080925301125251,14847468876278789756, 10377513966858151960, 25009495063667099])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_11() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7789585186810506137,3142937920393237003, 1333992290293988007, 3405117709433363837])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13941632647345873213,6335057726077175542, 4393742599438708031, 1166373703845352173])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6390806688290935667,12163093867902728788, 9027613641070652428, 675820989212401750])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9082582285834176161,13942698388996023976, 12899799278085120109, 172651649945782945])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8118044473652503376,3972860392159575080, 18001970451461929197, 2745036824307698284])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4725386799586988925,14574985519548118755, 13381024364756754960, 538854701215780983])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_12() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10199245236306392455,18140621211891476488, 4758321317734916393, 3144595063115386286])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14898452150870613763,9487660811820973703, 13785334839928419928, 351263958619809824])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12097172679406209335,14002882918970300028, 2452894575293828397, 1019803998844768462])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17022629186635173232,14815794137094443759, 59147758509713347, 1324693871018383428])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13003006006967630185,14689187664240821004, 10937745255676672023, 2663930550530023819])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5770922591673404829,8464599943760791237, 7093877099597617034, 2738298317153082672])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_13() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11109750628247559454,8796276775406286878, 11575195341794891410, 2389857922287860645])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13291332001938437743,15968693862995949189, 1331609001848773732, 845105697173110174])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5555725135990097571,8359307485281021078, 17868686353996897266, 2838730614700819494])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4036007270005950917,16917628883665513001, 1930259442491108057, 1741332669860380942])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5586692354362542943,651837647162159424, 8691242988124936691, 1385349058473676796])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2457040721467392298,10123092148601827012, 5912322787897649567, 1752588162218680612])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_14() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13776818487199148544,16428840051404012162, 14603768868797311081, 39026270245661099])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7371453859000933441,2748883790921104569, 16310673174402031236, 1649547947142404824])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16121307008453781352,8906102328050185959, 17170323266801431782, 1788962652510781744])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16877358414327449982,18399747750372623931, 9773243898135209257, 725551178318164664])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14713002840501485040,8246605639474105338, 229732633260237634, 2530982615656977995])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17357685569203809992,10404152319868930450, 2604386299360141600, 2334543948983597560])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_15() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([270623735424609756,14918163881616265516, 14901762973229713371, 2337283132865193098])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6292997584620635612,2144833437084895902, 2134378183304920992, 3349347073273767152])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17676547966088076883,458753872429696841, 11761259982572545257, 3267428759105239811])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2601818228172458045,15632739716319475472, 7593190040129529217, 2528421024446889529])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([52081136084281038,17559799843074941902, 15222581854534887623, 3101497702803168821])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8837388507886040524,7044691055583544131, 2985446620859937354, 1889381831823009255])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_16() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9331137865173431579,5914118189199684899, 5777934102313478341, 1570348050393075155])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17058892582591333113,16526401512954505844, 6833232030632977913, 1570516692309383007])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7342966309815578438,7537828808518296880, 5113539973391917665, 810797381146315792])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3323792432157577081,2371595650129315298, 16221184478425155365, 599359364534113446])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2335360793943989129,5186525376473818449, 675441685282991805, 1469820100822170111])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8447861390500647265,13887404727194987414, 7208527949506320237, 2218806390448196634])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_17() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8504167964167229421,1987673739983219397, 15695548489679160881, 1269388259138855172])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5699164409608181082,16943870829256571249, 642982170715230164, 2338598723983685377])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8041715062708232481,6181628535962690722, 16687549753794234249, 2408128973977932429])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2642837507779895735,9929455706859507264, 16516509058985299986, 3137964488796391471])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12562973452466521228,3911010946243569277, 16590246008846385690, 3416585279284380113])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11979507484866195887,17305143414537139887, 8613122189781079270, 2945902600826310694])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_18() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9333838327472285682,9617868530653233744, 4816427301812528335, 2048661490922076411])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17618791627439204140,10703376339490122766, 10683950770439504307, 1218489958915303876])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9243463686563249205,65292291085296869, 7672861835806619808, 1094869345595923609])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10948739580193828120,7308298784673390599, 11981784362067666487, 3240665762926479152])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6203610873777967397,2738951366607438922, 15888588011766250519, 2916760111404819866])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8262436935845292140,9312906657530830944, 6342771037362748205, 1659944025525149408])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_19() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18215517424930349890,5840556657713931368, 9726407883960506993, 2733222446438796815])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18417881114284240262,2482139030217700749, 12651144484493938070, 1778102854459179110])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1917567522355933632,3472920225926259847, 11627235905984107810, 1299194979255028525])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2734437922946594316,10025277329724169898, 5235261717006898493, 1327260968417511176])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10250183287278618915,4151673337958646318, 17710617657432679854, 293291523528482704])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18320702472934346261,16514596468241182434, 16851076470380498635, 124666343977784341])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_20() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12524438342549689577,5459481667655562933, 2221836122553761812, 2705195649187196137])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15909703146204720789,12131145516101663775, 8211724729294568100, 1505061990392751464])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3226095913684730576,6186281304628777765, 14402556762457030217, 358278747336173175])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12039415121966808908,5900712388241029694, 1602599201193736784, 1815031710939884417])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9512238475105550544,5874643446923927502, 12215691537564421078, 3056989570116418167])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10175333386773080829,11724856316761978061, 3360793854081019486, 2320196409143829544])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_21() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3448963930128990034,12901635784669426942, 7828660345994972202, 2116947423036271518])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9414229755864789752,17757293741410830478, 4382661608433947301, 238452403777729495])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6037166602713789517,16155624344065464919, 3981335822407313408, 846547834482100606])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6703252619463926074,11776578003599003015, 9239601730106065788, 2065465790354154486])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5126522199413229057,9866650836818007137, 9913248993236841148, 1875131280615191969])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17103899062020561889,16231513481229464812, 12555789897688984844, 2304275733230114852])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_22() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([287109964313747249,11413262288885818636, 2780078482931684748, 390486440201454578])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18441510591766573402,7220632814781873578, 9367905646664557674, 50888335744684078])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5123043798498298775,14258897169578363292, 8639034688359796122, 1307358757862109190])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9949337711694557619,4190678447805837035, 3030679323270283036, 246924042674169612])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9612367799524911901,13944124324546598759, 3933530899587146181, 3447805002723971132])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8480116776824046264,12370791869108421604, 9243542121437243874, 562931378724632129])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_23() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6819347212443508627,14380817956009020122, 11490613643786868874, 2675884994885701690])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8539140479632408989,8194211470684079269, 2077135028506898385, 524589038608530199])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7238320634589634943,2810869298491860188, 18190823316940059167, 601328476248575923])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17330135560310379025,13326842942671423959, 14192570876734829607, 3187189158523402450])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7347441058293950809,2381654992628648694, 12283127418585427050, 554127190895867299])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14525920750940424436,6251376237155514255, 9119745497840928963, 3026137110213085883])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_24() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2415338884681002846,2912680339196872922, 11710442952716234915, 1744538961600730111])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([713726665293542916,16116446517150523815, 7301716985417093672, 646541849301241213])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2802651880837342007,15264407571275126130, 11482219452212366791, 2580029176521980262])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8051384972651499709,13509097628089186570, 15987139753249728088, 1724821564430626417])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8664415540163095049,12168471377940660156, 10660581930556816542, 2985417154117979585])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7215323165745287777,17512809659064767494, 12967761364295926488, 1156086464150001555])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_25() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5847020509332917002,16107840255528183857, 13424777394943063602, 1118213906854175275])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7302759526184944596,13438215651397792756, 17503339316445158656, 2803489848961804495])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16118693761543242214,539489330238339253, 8780338381895319476, 3091216389784702423])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3819222424053953391,9484305177404131071, 4427467557402947400, 1081013420385521109])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3197682013228612250,14100609499874712157, 13041169619542001726, 3150877883620438330])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15031901740242069817,11552635673714776172, 13108773350990590777, 1584779323744170842])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_26() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7736666091845641957,18358818191766746013, 1679233114620004144, 375335990723118298])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17733721025036990864,6393003824585435020, 13936297069596663203, 1893327663359801799])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18093693628223309491,17936637473177723327, 8003761580206033133, 2282159549033336541])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13237504704449068028,4438394540129058755, 1419876011796052562, 322554199750204231])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2749757163456989388,3040049802220625708, 12838925560779984220, 3418584958513450119])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14131965388775222832,10546007623566370784, 4986128003727216251, 1822858366434246446])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_27() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16609168794555169316,16493659075023120097, 11085579940439096654, 3058228494886721962])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6282040349389757097,14665126689889831816, 47941081847894345, 1159320964236639124])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10106442699566474898,15481463452236569372, 2216549774949871936, 3214875185114687074])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12559738513751922886,11635511430638918878, 5707967392493025109, 2697916725532091242])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14166363440832640524,2696326784354862706, 14232084503259944463, 1343655363295586643])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3968375579488864249,5891246078049120946, 5881896279230690117, 2297090914499491897])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_28() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12796089133672114267,13701325530126098697, 8552244792200768737, 251608614261995397])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1739651274117659838,10383924183274804335, 1232401508593539744, 3192787264745467455])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16131052045340976757,7521662895573958583, 4066729838210315437, 3246864608797333303])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3556682161449420215,4447305674296501708, 16756527478588630418, 1611660486408356589])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17007742788691177755,8528942540112163609, 12106460327544228034, 1689339426891782604])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17788581739690298860,18079010206976808141, 11311196041425439257, 3128397073005990618])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_29() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7399514695810299876,15915840024206767881, 7861964664907148309, 3128730485594524316])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([446723938111159206,1287246602372247874, 4703622397805887749, 2094720261554967625])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14360350713707645204,16896621100042785798, 12877008806139365476, 856443816251001523])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12227319666002125525,16680200133876766018, 4570629093810756863, 128061889664448605])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7638453536790487049,13445264415522465708, 8959408682309985432, 3425125355783343221])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4941642995010997310,6415545896376924726, 6286639352509907220, 1166330931770153584])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_30() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4138536755720428249,16213310326368166523, 14764021647284744860, 2364884853001270249])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18155959633571591871,16139433975535113501, 435080588100196060, 910213465518351194])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7662805989174451489,13192898642512506288, 9837691826938349091, 996548280215820609])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14641212265627999576,9971000290503377956, 18325498896227450631, 49306008440482674])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14987163974283789123,15369367139258711381, 4805108040536182164, 865389779847616075])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12780902668628266944,4670264411791925444, 914787537407591974, 2802652746619412481])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_31() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1543219374122905771,8090450085994893608, 8765342238743279456, 2645906120949195560])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10590034945958063776,2148406424358347708, 11445056787421280387, 210343342343922388])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([84160011354238373,13765682145630609955, 7800796773103963386, 1406300471758320952])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([710490933452581864,18193564024607700386, 3400943045746472605, 9575821229388628])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17760125995096386084,11664557531912333499, 13522375229170762481, 375777927578750945])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12565086070997357285,9691086342514185574, 11509833649633872030, 1610814813808774869])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_32() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7291542090865801949,1577101696128872745, 17460728556119018831, 1481908667538396308])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11357486338903428945,9402931630429116071, 931417004746301649, 585421111836374602])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18076471528705267126,5349588559294005231, 9256503960246701550, 1811180944784968872])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8344292930152609609,425056255882468959, 13425843550653486842, 264791131786334434])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11086156574821929778,5292553283741095858, 12089065119319887245, 2319694087620793161])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4633273787664257371,7547310177033538971, 6601454254372768242, 2712998898483652673])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_33() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15409796379905470930,7552359155329271944, 8707799418017204781, 2077799144247017100])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1542392928164153258,10603876708432473252, 10302427976007626990, 1603620034839836595])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14244300257937697759,1487105642315742749, 9475997014904662615, 904186338103397177])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11713173046184750089,9160977860234461357, 6078171397054457180, 2002854249002367041])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4780673991028727399,4569514823412893415, 16683474281454486792, 168860920449206376])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4445516531299081531,4336300495570212604, 15119934909769648988, 2113658241731487389])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_34() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3739053348046568870,13759850316338548409, 13717292334514285466, 229901727331435613])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9837388131089352616,13837864382286695963, 16647339903286215086, 1392188083563275559])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5614503646084565013,11539317751278873413, 15777690545326792255, 1179853794666144299])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15640529013230252674,17167675400528452069, 6341480850007228862, 2516290777480162977])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2568985754653549489,8331258617886507398, 15289896141672320485, 3111748989238118641])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8520725304405802997,15638238802437077320, 17665258704505957543, 2731156151271490823])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_35() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7120317261476557715,7552257997228182021, 16358236190799863642, 1629867967602944431])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2926655896310185389,11342398253884210994, 14243889332822464811, 5673933864441291])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15263597988564605707,1834204502393602270, 2781236779387804203, 335388004383960306])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13837667168429014907,4675266620119564585, 17991599336536916282, 205128786167620998])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12226457088307444571,13281439801408605565, 6871025939324737887, 1355231828771092400])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5848111637689269596,18344987805008892157, 10290344629027789717, 2394074559139099143])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_36() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13555239015132717788,14924527318658672652, 9612862511435704261, 1788235917197541149])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13743185498493428374,13042375823225200846, 7761942357328267608, 862687485163677080])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14465100373986763913,11398519268715632530, 23522222063967972, 1057670260062133535])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15976699050801485602,6319788719106771266, 5566995811024330042, 2387504266708092562])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16569113230526403569,17725514535345950488, 11866006831990305753, 2130996117207853294])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1527532136197539549,2786640387952763279, 4094770523233825381, 1206940085346694695])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_37() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5114804075405120203,4172763077557640727, 3328951903873692349, 472711820379981624])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12486348542366902455,5733542869038903260, 9171736716159526893, 347286838645629371])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7426745889640868715,7336034198735935872, 225818981130895468, 2851352892440829973])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13558352020304174794,8731800623672578152, 8611997025371520380, 803901775735360605])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4306725704066455433,18139295094576557768, 10045251235784233754, 2086969227141113700])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14119044221441020420,6574752427733012334, 17898012107281779637, 1994133102098711084])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_38() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7155714555768312444,10481259648759533641, 3320847119085430469, 735361489356423879])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17450582325843748451,9873542994348163314, 14772659291204377472, 626502329742690870])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3371886791869284567,15381838635685684107, 5657371297269882352, 2651932389293531103])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12269515269623735501,8918742107553422877, 4172805128451843932, 2733666274434653456])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9573118767154288402,15470856646598692794, 8105024619525140398, 2224539222662921593])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17034452844578716199,8554107061262379270, 16591785574730849418, 1633226586394386861])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_39() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4711161931912789291,18130371323456201232, 15277646067184296809, 1342727550878387191])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1467126513716047660,8021413539023584799, 17801508658762354334, 3213122867712408619])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5853050774925202553,9705908432060068641, 7538740777643461165, 1804229708703189317])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13532391595908283977,2706364488996114206, 13664153023687637490, 2790608905367055510])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14519164874451284801,9104102277442927166, 13279729600879296984, 2861474850255824756])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5377949737843903762,12797117373748448714, 14215446886332233979, 2017226962682556292])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_40() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6267645080032865320,10750275706111093063, 2107525863779491991, 1968861127619145353])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4415519846160173950,11096921346300086957, 1170710085091004866, 72616782227125758])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10232906705228961780,15876358800657746107, 7449296634528376275, 3191557894907054641])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6084185408544380217,12166250516002141475, 2699655953210779176, 135679131148756839])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13839678956621451371,5837800799471155642, 13017709583009916558, 3397622215689528221])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15500223681764224151,10301813571936861125, 14855805854578839215, 1474196592120957640])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_41() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6159919441405191294,4477945359632189959, 15198662699206172876, 1280396025645841436])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10477543762425302075,7064537348438568871, 11743696811261499485, 2383919542106740699])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([248751161004592530,12216330140153424401, 8543578797685815603, 2859027025838042733])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6194656446907457552,17876752927769885539, 8407565900741576915, 482489911112910917])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2765500164015116551,3029999966476280962, 11243794975661292962, 860337896095117828])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16994853261164388493,10277461362946878297, 11170518452121706239, 1160792507953008658])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_42() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15529561912129630379,2206068326658408061, 10937502935126907161, 1810329932763296750])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11773988251929245535,8168905259549524408, 15881146625677964247, 3301148154720107604])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8727185441572338877,456204486442982053, 8562669116275066184, 1105708298348662747])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9870364555866928692,232546030253946073, 10620863432028529411, 1618420585265274997])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10112613435221297756,5169148670295997689, 3957217890151964010, 2979695178810704664])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8131745237183518180,8781822738273576162, 754287471595691592, 1808012455343298918])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_43() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7685420508597090459,2244927223990886702, 1415534581430209074, 1799099260139195975])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([37517095230738605,1692194596799775857, 4156007241714134993, 1625175245680050152])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16597362184989623665,8874879443557824057, 4575284840779117537, 818887851400572696])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12080683947072644023,6758163829153298377, 784679329732419071, 309203053972157318])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3952837571701453180,1903135870921686260, 10457117118987937084, 3405423714905158163])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15017229640277038985,13803203662891018815, 8747510305522008880, 1448200982938534942])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_44() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([556304748214622935,3135283039681552363, 16865373005037478071, 2938350916681646433])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13050139955487689154,13333430870083485265, 4329060736223953731, 1047348912653006317])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8770383764551582800,56366096034035302, 2685154886001377122, 3249409559619901998])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12884160568804891593,11581608250361043986, 14196902620164457538, 2578710095761385912])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15546851523730721805,14092579518078533720, 8871348309890717741, 294049289375972753])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([342085843141481611,10154361958468512950, 11174549408220543679, 2893600745735005009])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_45() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14647082872060818788,14817078032273354816, 10064634629631738490, 1606188032088004980])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3705444889065882940,7361127136866573738, 6520054309572279251, 1051268700445219106])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18336063286515740324,15224883798088497869, 6278436591878330149, 2163055900135459124])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6793070038140550276,18017444111817406813, 6767301765502429892, 212697772470584297])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14535225152069889826,533382451973379504, 6614622700216502268, 537210752675714696])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([214839826762703769,4686888271532457721, 13579582382283685089, 2716294784014812582])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_46() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8355857482047942794,9648467663681812189, 480005125199695496, 1250818138525467346])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17343404999127466843,6425518906437539942, 870875280672844244, 3161873369949566100])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17907793707812022497,10691554820921122451, 1280459195158690356, 2990099148364492831])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3617438327511701730,12341708758349691462, 16098706603763747897, 1436467705391832432])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6862210410128284842,2985523853272151856, 10472690361103266932, 400516676924338298])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7221846984979338584,334208652697608704, 2438082793402141192, 1673680049462395272])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_47() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8428076325473342447,9130298170338116706, 14251703806791933252, 452784505910647687])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12548639873433632870,6647155020402947175, 15808303000429910320, 922278877994736356])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16670237437411228134,13110053254265281925, 11070339283954752612, 3344003398560746676])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15344203836219615739,16450005542301114345, 7928336079533256047, 3092930395156423900])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4842364196863981737,16835448445746375808, 17822749353942278677, 1766986132760433167])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12903021811818100894,17392054340424039541, 2970692416086698943, 1092678685318618832])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_48() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6053571397947271628,8986382231993405489, 4470814686790468009, 1670109159964864270])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11670954415637520480,8676064407044588983, 3493307303700138283, 381991932643337351])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14919872639259527266,11204446196210274807, 3938746915973795236, 193071420051602668])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2169640326393674147,2976414262502596941, 1068741284613745466, 3241248254328953421])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3347050930566263179,15227623261940056373, 5064449734588130975, 1644841272216980058])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1625280631411413880,1702262130639902885, 9529161201018776810, 1829165212640860034])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_49() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9946192486741291674,16009888218582641868, 5258189543169640418, 2800620383260601224])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6260910462313486021,18019731693449785283, 6581400544059660888, 344253000900514168])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10913968561495584788,17286209953281281921, 3832380249816653571, 2296504327540868255])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5980313309144068105,9676642392212827000, 12750006247630515173, 2419885455009742218])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10736283731559531821,469413005744342390, 6693792377875160787, 709913673382984383])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14260366320606871896,16001207483173510002, 9418928439988216436, 1940515962799049672])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_50() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([668612708904202850,3167321822604705979, 1579530902783930144, 2346397967698798449])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14290763155749462758,2407447641980493596, 15407234844910327919, 3384970810715598847])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1177380238022283512,8961579017544796719, 8591946660150039578, 2492736147752479232])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1066371122727170893,10595411439863435174, 13669654662988561356, 1536218391537906349])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3990287772318266996,4126143702790782387, 12391293695136844386, 2226590308183300015])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11474196062101036939,6512232067889266870, 15299488138452155551, 1359612874814541172])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_51() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7833810580934822787,2278173342061405224, 16625781782762714556, 449975474420201749])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15908676400454436457,7659075249519984796, 6092125453849823626, 2347403061601242932])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([704334472188542423,7265255395597411589, 17778200845973254481, 2916194317447545558])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12120008460566880578,7050874281741056724, 16913640816487850711, 69787359441453702])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12421985291619175106,13616740946265748115, 15690972551032352715, 2976784100392826795])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13053194126554283537,2221797547088586530, 5087119149953290972, 2606351311925567415])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_52() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14910168874986442728,8031767862594161641, 13082123848268012237, 327033253264007364])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16437551873538962056,9555318888998921550, 10441499694453588872, 1216857201682350755])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8929484257290050721,12216641768225203374, 17797023415867847222, 457776622682966994])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1003814703751094379,8402320400688343081, 14095669723564397446, 2701594127802062962])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1605828574345606971,2282854075343552347, 5322406924554484809, 1772824224631335692])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7578546237214991511,14700451298734272718, 11949868685548483703, 612554363523646164])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_53() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16667485799333773504,3221086101404825275, 18335055350165017628, 3220724970874634648])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9965961477366606014,9514984276782648940, 6659055027309226244, 432815556239122909])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8871475679956329666,13656411655313599985, 10278285147808794900, 2104758402770576031])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1881275745878281923,11114772088861627375, 10479992110691850233, 463976086458283984])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7107780817631756592,10796550827481526984, 17689566673997668915, 1425588873985650284])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13455981903192993053,2524251370558876649, 8142796561702863700, 1489982824885883810])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_54() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4608410307150166716,9949176550436589210, 8414886195581298390, 3351545680158952230])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12614760649397657074,10333447317345225951, 15068923264347419408, 944617349099394366])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7315350430268972497,3548934616792043495, 9263354149466389875, 1806153137917315461])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([890712979474667191,10947174018244634898, 12411878852338739934, 2406696367944098355])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17753449963041275052,289830216119421164, 10823988019836381479, 1864433638483422165])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12659154223165019981,14291208385812288033, 4211077010581610829, 405180633523493395])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_55() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([512316247260094648,6786053729138575658, 10061032147386610763, 1327278236519464884])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7775870594655883823,8686737039564117681, 5138930906871917634, 2225217506801160330])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17678949499006414264,3046943661362056954, 603782209793683918, 2949855076993879063])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15648404799849869824,8852492324982358106, 7236352333723893911, 2968792539319848949])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1341824392415388245,14839234163860451146, 10927154602245338474, 1723556217420853083])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3698733681168442673,12520078737395306409, 6552099471998238285, 3297070465959638818])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_56() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9365894875577831369,5088926746804912808, 12206089286387205189, 2720900606756380239])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7869130394231972803,87614893153743506, 7398100225563446544, 425580236367558222])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9623386052275909434,14136039815193396738, 1039748712468975291, 2703775407007442287])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9874018372711419703,17610825206743430688, 2030382861131513741, 1089540355975414475])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18198233620107185292,8383715319697940874, 16092961870783587833, 3059404893348632448])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8030201406162679647,14390016679756577003, 281274593549247080, 3355147701823212150])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_57() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5422680240265832996,1946318342117916306, 3664916472817012405, 3241874286956908535])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18311043380692671418,8100242697301145939, 12667924700306939342, 1916567917399583004])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17950967283822921384,16844242693178970687, 8067610294475730584, 2973722185713614929])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1586896194752532496,14530098787235041139, 14057579702446203562, 156923356255504831])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16355350933727732631,11776541475409484475, 14261559773007539506, 2751337877015607329])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([828737256128366603,13583580367718593264, 11493988924326139353, 457769640694574808])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_58() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13003328686856013800,1711598861164327991, 6468057120367482348, 2730935446791051989])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7935427476618081549,9348338193126529088, 4081002799623368688, 928439523363955439])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13002181057977534922,2628453963212504637, 18352375669292150337, 1235495029864377313])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15338249627883426984,18007053840077985809, 12604282715825014794, 2692787787259905194])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9093686028111629152,7994041345650666007, 11608789992358999520, 3223982669655634038])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2992640776169583751,1871142284957443263, 7623327292315785978, 1387205035875433685])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_59() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5463865735891083899,14630160823257816750, 1155950244367731559, 3099083154394233077])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11224215036972939113,9261469020329846904, 6881454910289088652, 671621440445142178])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4421356026529641239,9446130041908699392, 1922014958537615400, 3274503252285438062])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8580277962475378750,2554147701309430660, 11589922152054583084, 1644363429981893214])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2759293219903128844,13723779788575808075, 5429734218610420872, 284270640615813289])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10127298648082168099,9775834864860166971, 16034285677595279060, 1969785921159627744])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_60() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([600580334032652804,2321757057342758172, 12927044650911696839, 912481606074830813])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1742222540423170356,5388411484089577888, 15669036111913009800, 2290353886734264530])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13524932709418747676,8729729059288064300, 13248815784412083209, 985601685628940268])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8095896670723043672,9479714206220833451, 9935357687700323105, 1987935092865768071])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12351501345222280255,4564669834210920077, 13079238967595314121, 2893588379208385448])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6790684858368175968,12342307814734069340, 17437338779261382151, 3113323595042518587])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_61() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4335458208259077364,3070146124880902986, 12132813483765417998, 2189079783051835915])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1273417055656037261,15242501717304447765, 7470293015109889396, 60979368330265425])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([495352304476389410,2522603611861727223, 17400605418483600773, 2405243173227662275])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4145492522477043770,3930196348863807874, 14033363453362515818, 1683218100918056121])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17948501231187937250,16236352245361706135, 13876518441607112130, 1989605743992033509])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16575051902264652112,3638918838457874372, 12639744488953793639, 2286623855733929397])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_62() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10038236537227454322,3834487206512150896, 4837315834914812659, 304845306814793610])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14889474274324897851,4412684911204420504, 12052436362387123584, 2319462749387062806])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6844925402023018696,2984767693307730228, 8452064398111784330, 2380067291807988977])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5724892535103664571,11711437062631280273, 16270950815446121735, 2789330935070267190])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9755397992000155692,6036355593694428132, 4635001509125872439, 2886159028675813399])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16069879372809196519,2291312007277523126, 6433180933700077439, 1486270900233595497])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_63() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13921265890829940939,7636330375160570474, 4665960856949324823, 2637580595337034782])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1676311535076855475,14388116664785243967, 6999886116491182694, 1225600792830782493])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18421970317862919207,1327288065051737192, 7012137319813403714, 2211320130364568615])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17479704104393317052,7470349539550992410, 7224478043021223785, 2243932882698239654])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3112539271062938499,17833733510041471540, 15283182265851972375, 44533906695271561])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17205450681903040044,3522788048615418771, 16922755076551489721, 1439467848016457250])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_64() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13377146695259432802,14100753620496409992, 7674379385896553691, 2573832464465008930])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([557324928434175368,16981228059930419063, 6030243056256668828, 3417519645769543382])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16569293480025381657,10061462583010325847, 1668892605250494803, 3372783123198145055])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2313789391118724807,8732181255536968160, 4654984413883914588, 2797945300820465029])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10786873503828530253,12816469824825706583, 12612291718157215005, 765198871954081674])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16567936393605057352,14195469192344509149, 2677336580189740007, 1591821379742287238])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_65() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13626726796417775597,905039304520630014, 14385262270270118529, 931464122089285366])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6459046813597811608,686563215136329104, 17840121544795996864, 2526159168346761237])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1740725520778714189,3970588031411753937, 15138818668422454728, 3154065812649955101])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16142871230366316420,382147931728803859, 1578398685310446495, 2892321005796480272])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4573347424348238098,13930425395157667377, 7547321432605579256, 2713293223971936786])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2939755484115294576,6869514172505015788, 7002476311999383824, 207732096327248456])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_66() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13709442686358577124,15832464831351649571, 12076039113892243546, 3455485297433323062])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18023010755270710631,1345091752456542941, 7706395518894446797, 3294337764106111222])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17912043887633553435,3081559084720998117, 18054355427023015612, 3134494175808145082])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([186337887500567930,7222531963656893566, 14155938318246282343, 2771351750031622878])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12609246564220273487,1594397207756565949, 9825723030124067254, 2724717191726459635])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12983994115142587269,976540368811885047, 18335483755257327302, 2167427977713783633])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_67() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16767682901827683841,13701264828479258978, 10181214404393679883, 1395082227297335398])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9978352420098653778,15908682512986432460, 13661981653086040843, 424470769782416566])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4767469812439913295,12418131412191642916, 4756013449213123766, 1564404981129552671])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8809079650558686821,2123224115614886271, 6313393965193212986, 3378636516059800730])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17072499987405276473,10034793520409945765, 15759239356590162154, 413675290036181673])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10403544644649022418,4260743975056099883, 13757505939064072804, 595280312120297909])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_68() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11927676530117983136,13988462498903978459, 16181886719869833670, 160879608735390606])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15764181760919483149,4749684855409639810, 3265710675173907460, 1373672660229324824])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4702490859355290177,10088008658301722056, 15951539533040514730, 3112554746247782292])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7717613784005755383,9824452686697220968, 4108379304217917329, 2144271069510608047])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5210921203500069385,8541209335657447558, 1626519623793301518, 2000565976104463586])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14157975516393092985,12099147446097414594, 6648131713760764113, 969754343192134911])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_69() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17980189460187830778,1889219941237090783, 13909195107060244038, 1685396046443038178])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3051833930183928347,18416720040833329799, 10087408079259648135, 1576906038236231369])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15466516961608624603,16844094941723239288, 11285642211676663926, 2975145632133377167])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13471066094520601513,17016282780077945631, 3938930665007322204, 708245874277978584])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15031906007597449965,6239039503303329396, 13836318553878840955, 2695871689734768577])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6793852692303112306,6313229706798686289, 1376699463481876760, 2357506978952490562])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_70() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11366828076657158452,9549996211954041848, 3650044825479732830, 1373814187006603594])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9692203809937244817,7673331868786818439, 541285382427134688, 2059056691169337877])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2839986922434318894,6239121690087443138, 3236924118009391825, 2074927829763140595])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3579273526184828578,4647357847463812081, 10661538303636942038, 3413451426506981126])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14254589568286730706,13380399875864742632, 10230702462255046996, 2216587113744176350])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12638907948086058139,10816299396480670272, 6983807940519337989, 3230703026124390459])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_71() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5058092415032108019,8462765089004818857, 7967880597174752471, 1490019821615785378])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14070960174070275357,11842197375827090414, 1306527162536347535, 1957376770316997198])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13198164446365042680,11901445055653796909, 2281480632337590920, 3142146612845804764])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4156556221081109790,9039985719708943681, 7223578382291564494, 570548833514427866])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14506878757868128098,16544943007052494589, 2198031624981621405, 2033961111379319849])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1569457014480111647,8958345506216933074, 3065879269802886706, 1556744670853561648])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_72() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8670769452050281486,2603157643224096297, 5170940931522518091, 2169635355341440494])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17858655563487727388,13974350819178327018, 14191523303038937084, 1933479454295296944])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11561363935362204025,5244667953953900991, 17590897577082643744, 452292303762752503])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10548293481047491397,3726803642967868876, 16004578249770811828, 1197762234507486424])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4825160836412796851,15796880338500075246, 18104269758408963704, 3351107969528331429])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3677353594604656777,13243886080375460536, 493482802944247320, 2887949280159124944])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_73() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8479760529281690674,7994751242138411134, 15345767225237783154, 3430220711344759297])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([524416989865606757,904006822949417225, 2796530985342658600, 479884119105635171])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([830443950718677975,1025634055668035284, 18172819390113448366, 131764769808948612])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4222005762950425160,17555140366314449330, 14403806003810144495, 2669422862431671005])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14565552002431377048,16565630375138188061, 14575995455443639551, 1274682148956826002])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4889098274062523040,3593520692859002798, 5392306450307316027, 3182134861099261142])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_74() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1309357342976202953,4214876501554558651, 612196342916926809, 2975961936085837136])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10269679222705203257,11780564713707960686, 1794612721178241253, 1534343832212062965])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15678624791121454939,10355914611054688241, 7599887174411926085, 2863204088224670613])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6002955251576793929,6234348676434665827, 14978427608665948614, 2268876856796670138])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9257438800131341756,3130394428690402783, 18230611937698292340, 2740678391300670167])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14390331293882510601,16419122494332313410, 9922013992643156140, 2844396889136699083])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_75() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10098955407197731028,12656715286844168905, 15535148986134773930, 2947605084333243272])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10657330263306238797,2217315742359539172, 18265949491644139088, 846602167354454739])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9038037631799291094,5225532145101885408, 8281059509122928437, 28324414295733782])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17467972194800785766,5536291185643948069, 12736906291079283085, 1852657603193167013])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1516587024314539810,11982255204610613517, 1155001714131273471, 896401992504972885])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17030494227113718239,13630795687692713630, 7460998560730100552, 132793221639939795])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_76() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10176615368683446660,601583534723294556, 13110665065271052236, 2861720736213065106])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17880345649508354921,3871979978980853183, 6655522217469181008, 200518909375314463])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1798745044690121375,10759244287798089531, 8948551391234015540, 928153838998254791])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4854644120111971641,11604223912110353503, 18403836121615736198, 2037443035641934904])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16565873182373372440,7400749194843479231, 3008657753327233814, 839805860278934523])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9378001881854219365,8314728862485991704, 7763929823464409236, 1843342585299060254])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_77() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4856522731738463993,12034688578446946381, 193602104566264668, 2354117359251320137])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13090388147234528083,6770974428891810482, 1846596488981736037, 1971220940796455096])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14016549827697015028,9608312358581675044, 4938637631394377688, 2612555136020187704])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3767156235301953140,3708001549184614692, 15817524667752243240, 2711889907619030045])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1325748209183447016,14610883328180414904, 1759057113030637034, 525339018639169178])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16428758989474667697,13196397865265683154, 9151441209947915125, 3283208787367466898])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_78() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6553874684694368455,9301571099664561278, 14237282328599277535, 2471261460831592801])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1636100556647778311,3827957952009907610, 15551048411498001302, 1898011813581247742])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16950077254605975014,14754718289583278266, 11624264081282056163, 1305554521772707728])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12772965760979088209,394669632841202750, 16394036464624281841, 3426583584086903179])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3965499529118527969,1864766547587760102, 4921104890318981551, 1038463313152890140])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17072525794739801527,6022531744446010312, 14453564009875723049, 3325175676479929237])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_79() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1090926775047554870,7374864286345996973, 5785741914586266972, 2276686030916015238])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9792707039150846344,4516930697246621247, 4448084479452425457, 1996587931051355115])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17379415007087396391,9575908516321139397, 9530433683536101519, 3122784550798977959])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16525951791268729871,16200773062959914622, 17199317757738025354, 2450494661415658234])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17506600053239517039,6744584825066007595, 6840225580237310845, 1171279626464492107])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4265082730179779326,16653451769088725385, 12829350476060252712, 1346333834376613912])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_80() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7432876191203405911,12664807927281999210, 1595364054487489041, 2701447910710025167])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11940602867216769464,7627319978759996784, 10788959484739266130, 1949685945741268869])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3967213674833130328,9488545194614397643, 10379184825751751890, 2121056850668979323])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15678207676027215262,7992288253016774215, 8771697536939175083, 3445525118491567819])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([135614741216695967,16195561188624744339, 7911415893389956452, 2790743246561021932])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6623361901228602847,5489245354088947357, 4262256575266375647, 162980415131956532])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_81() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3519293655442125928,11780008180724893403, 12228611059897045935, 1697727080360643973])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16170302785219017980,5730153183903731472, 4020348276203094029, 366170179338240947])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15621740143426463190,7449733180026159217, 869406022328016112, 3219751642499375377])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17387351768998469767,6892753790464412759, 11476027800764791449, 3178641352535219284])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14840849188449838914,15625289482808013370, 13932615326385662473, 3016388555336841699])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13737340252094200359,828424022266252818, 10426451617748214473, 574784688313300629])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_82() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16227214634012445639,10180816771680461395, 5748796231136300764, 109165530280654655])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1722301671187689106,2808728941222518632, 11092363639505192063, 2725335253653097289])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11681521770028146767,11672153066658936838, 17998965641137332885, 3151190869304898104])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9099300166845450022,8531745995743027770, 9899135786717951957, 208878410924132320])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4766498456916239682,12673053865872984467, 12345543418898768255, 474193184295504085])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10284844796759031945,5244886598751168886, 15962087395817320263, 2880419048435354140])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_83() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3583795318172233317,11268429957831117231, 8134382223248204832, 770582330385718857])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5445011628784955794,8132405898873957770, 13207376010329008512, 3022248363674238333])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14806908376731975256,5676584781280515677, 2181484093012997946, 47417766018892056])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4534754134813343491,41333861970492782, 8049221930611838086, 1061618236260490291])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7633387908047997279,10470260179857079669, 12862306077389804594, 1146878657420453056])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8307119838564452965,13272869246662139979, 4002361588242659082, 407514155588243603])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_84() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13263646417427634136,17355738782685760364, 9357544541436672632, 2221196746893269601])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16541889920093155845,3879754979833245024, 11772995882777749148, 1494809808540438513])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17276331879514065224,7052008681533180528, 13373166147395170538, 1416611334284347550])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1583651981840925397,16640752858233130827, 10454241491761773452, 3392937361912669238])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1353976248177513358,8562732089841534050, 11765294704957119933, 1810876761327035348])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([656799983739244941,8068547632821329446, 401210878746038035, 617449648840504201])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_85() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15439336233301813122,6708743064136549125, 6956120218660849920, 1426788904589992037])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10482862259574859089,2720795240313759542, 13849853619347396249, 451045416005369770])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16629460883034211643,15841847014574277074, 7307152824076067709, 3220001320002591522])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7364705659418664452,87748415856981029, 884386995931112496, 2978143857741552980])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11607446160836332799,5935320633989379015, 1950226349158249541, 1477507517931399127])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13643865300084526800,4288803503352663118, 2234848628509192220, 2162603572046936857])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_86() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4618868288865848009,18154973364001707526, 3008530448383180123, 320132757662851989])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2679164917805814420,1582666986058664909, 18104076937350696389, 2547159725166493304])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([665923590493782918,15080850902817201290, 15247621324780990287, 1373009601597800244])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10136008735044576801,5925439540950230479, 4926715132399968983, 2379420566225313231])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4561965519654710606,10785850761679807080, 43372938273663600, 3414280957004115003])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9244942981313276843,3041430537908235615, 17139575369328235653, 2256865004348951090])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_87() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12022285821251125120,8394710227083158936, 9190625303714994848, 1316584088401679566])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2648153319109453592,7281114132632607450, 4723070212040088738, 1264676797495982032])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16868530150508351437,12130043265208003121, 13122728819502030419, 2305990981832655472])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16000751638757593015,18294701796841483430, 10785978653820750417, 2876298256075627220])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8640937735723485698,812414643428927659, 313595488712102211, 1576472057062325075])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14748628538018346012,16530758572193484058, 15555401233343268105, 2341080813301348146])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_88() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16880261864759797230,11149320296478205677, 13670673657387330497, 2008564746625684525])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1950835287171218464,16239277735248765879, 16663203488411686767, 2143460148601497489])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17668110190351798747,16869553882511565562, 14545293657630083960, 211851059569339709])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5286719363536782032,10967886215506487151, 10989156048966279274, 788469298650854834])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15938110350409015742,3964858519090176321, 12162238490720959149, 1789796613784663246])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14738972261860833048,2448765326472352742, 18020981669628922767, 2543466490105874023])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_89() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17710901417794188576,18236591538600929218, 12412898114312061470, 398462371967701669])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8909664890651079601,5380521210117490734, 1250490496235015662, 2362333341771676681])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7524201333331065030,5738359738418169909, 6363940170721752921, 2937552274851588943])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8641970889876524681,15112341560553929654, 15325233780144964676, 1970929248826172742])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([985343584010635771,13139852461916839594, 997942083546923325, 1200546018464584570])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12833498188098157810,8667921397909111577, 4027833419537256378, 530474779784853633])) 
 		)
 	)
}

pub fn get_gamma_g2_neg_pc_90() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13996630899790034017,1290716614717735497, 8575162876585456788, 987692379801789046])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12847700138400039910,11302335817046679317, 15327917039527460971, 1379901220959135260])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8107823658327067608,849344582515770278, 830549737769622907, 1862251517777338692])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8325223645169189936,5839473372025888201, 2719260694822918577, 2075505971877390177])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5131551208787955579,5038163076084876730, 15415259148435454002, 743545834700814883])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6452707443835080404,16592201127676679593, 1209024538326743467, 3342681632315438568])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_0() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14631252108350306461,8203480703259017856, 3888877493897289220, 2844775969426548285])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12000420428291277021,11496597742785733435, 15273382494627258982, 446128726303294437])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11678103582966451863,8016069214257533790, 9768162986972698766, 1034983434672964834])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17221547546422010397,14089453717079341670, 15167745250981482361, 2067008011610073807])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17582329613473998042,13223203271519480260, 18235058756574975296, 1247122477364887015])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14942249620531812496,5896926345524142678, 13609817181157103195, 381381230988494926])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_1() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3612910817356811193,14995061587700135220, 16267940004579387715, 1817979552883336130])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11308183436300381440,14101714653785140260, 8423599999941378164, 3349622651709249796])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([576211075138731879,7582834671613083873, 17183702691373460336, 1865895113421886488])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2376073478383032480,8381943640488993051, 3572028920571421082, 2241760093089234551])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6630771398796167945,14169322314527617684, 1447095082658078528, 787568610005823348])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2343027941947731307,4556801575058874856, 1406205701301952020, 3076386312553788633])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_2() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18030042637304288567,2516686799632119913, 8092632598173832260, 2907807480879373237])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14815049023852400109,8953238178097045675, 2531653261194920670, 1277702080360070851])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6524508311441251172,13929005196451006739, 3996787417794767844, 454731711162445305])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4453156919473692449,3792717582602802646, 7123820410671693312, 2537358745747395989])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8267676629564807572,16516811996963184300, 15675418030464798586, 2808670842562665468])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([773213943591673862,5692063772711831473, 1307483000680616020, 2337939985497136518])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_3() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5847050115358698693,3637247392313121025, 16900113447737775091, 3170183321651785212])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16161032499936009400,17854403348773562683, 13303002954457849484, 942262832469876823])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15885041212085227398,15517480464597028140, 7535211865581757127, 1234768082359336243])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5443910887202136760,4728828620352170508, 578836146553728628, 1256001215687798010])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8154536071946288780,9917548778840914275, 7967561630935427813, 3301035472428677512])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5619303576956086025,10064973846208685512, 14332668438041693903, 2377474674890139176])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_4() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17707508853734216782,2409354991718747381, 4078437004062965377, 2311558643459476994])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14921398932679994685,16999403597455362321, 13527894255428203719, 2115999181850078550])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9537743338471032683,7035554502925021282, 1647232243439068734, 1477719629417746378])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([948693852424705557,1964180981986337137, 2855768542568617718, 3466215919152694117])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10115475657254114003,7473115584598936384, 10445550037670707667, 2519334836220924998])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9329141180035093386,15329962550492958507, 5509456101988505213, 2710820035159028065])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_5() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8535480655141316906,6731399451106734924, 17527760263624445486, 298861936320757272])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2768573690003823825,5181731216194273094, 10508209808590014857, 1386722701575243323])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3763161516078441338,9852449246792138460, 14582744187527714465, 1147283021070833910])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17625430231895419492,8895867072741112497, 14730147280476037099, 17970797548303428])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7739283745092899962,10512797568641404807, 1502538880475803782, 1443328007679724921])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6792758506840880564,3441375162234418793, 4001480427634203983, 2487733805662466547])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_6() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17357355358302829812,5144945249926656104, 1092882828157637812, 558382059691031862])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3602660165178229524,16252440418906061643, 5005187902237588098, 1372839734811202210])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2752262875311629737,2487066043934985103, 14318701497021492648, 3259571521189568964])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2480888024818344746,10767856469426303529, 4004706784280622841, 1259383333531262761])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10849756438960658179,8811954544139683331, 18228950850416168960, 2680067083612449732])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10896844886761931640,11558265989102742819, 15844233361106968380, 122907086135042636])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_7() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7907075668926218928,11411058995299238904, 11524596531288756632, 2059645661477596863])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4549370680204318637,17752113529994505205, 15713636183528647552, 1443876291504969886])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7470657116950367921,2318743308131424267, 5881585915328235002, 727570937564228253])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7448137669174742406,15308378256384570686, 4887267989979079158, 1552849900913726512])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17918427665023828285,18037832510355850052, 11515431117059310953, 1473915476922102764])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1497856233033100807,515680367827496949, 3439155555293467302, 3446652339553536297])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_8() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15839037892922674359,6845251608008069754, 15817560877728534622, 2694876105171571354])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9930572464385417461,16227694469594594113, 16698782566995229325, 1935089759758506979])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7046010050008351555,1763562302759257265, 10008868872638283711, 791337509726818271])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18109878606876696688,4664876393492168533, 7019078391075431359, 2756223280411799160])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7998153388518657597,16933041591072774578, 357692035522329562, 2098886379659771589])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11605761739028175576,13217855528705192570, 10364695951848186277, 675116890122902451])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_9() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9945407357859895541,16815273827176847060, 18246178794352909357, 746703226000125137])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7838214127094234835,18116385119916873363, 14588549035420838641, 1664931691396577176])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7442457229473269967,15028763530504714582, 18396765842752804586, 3350879812730794961])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5218829890410918161,17487822011489561622, 7825817325714597786, 864410238537653618])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2292900736817135910,10149386616698974545, 632734746154783681, 2690972996203017325])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12026485972600482183,12218488362960113225, 8270709325335779841, 2491263547873092704])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_10() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2260698664581258483,14548260675922708650, 6395877682152137130, 1665834012726073870])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3680799541486575672,15986151987930862293, 2829387773245891927, 288047200844996409])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14411639029513231594,269007757790937136, 17338485476545832140, 214517866131134911])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9272365159485668221,12361056910572987688, 11339674445211700225, 3091526821264410557])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10112474220544349873,1967022390101425900, 12259330884478209315, 903108451378557508])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2668760940372403655,14378090001848951431, 9849376156731800022, 2087179509886415066])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_11() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7659356784077808053,11533806858261097774, 2975012816036755510, 2237287839778523050])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13735824467793973065,6273825156692536031, 12812380793668576214, 2622822821304335078])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9155799757144263013,8238960102853176939, 3705982458754620015, 116217456675381118])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16523183163420607426,15213195793064237822, 12856898061561019313, 2534734523235167680])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1118363442409191366,1512160401562770102, 17466405408005280573, 2943007206819621190])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([266579242385016722,5317621724935433977, 13856771430495799201, 2885699488611557975])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_12() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4749160792596909287,7901768001425702645, 10557459141651627705, 2140385925992348343])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15477989047045609661,6456615610011693684, 14780868447142673695, 236025390792526490])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1478524933466760386,10809030574087298115, 8506045731707743004, 3221607125713816867])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([129736263699793707,4597379669318295074, 11313315416093992923, 885610609663063670])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3396844126537774388,15154548225244771354, 10160788299834390033, 1930318365819283603])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9888118372297519621,12529711879140080877, 15650155321294487257, 241185781603815176])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_13() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4324742802683880874,17602557016502532318, 6683893840279194700, 768152887390339482])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3598310115214331837,4284083482117136598, 8280463537389100246, 2217114463929775102])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1324770530355377192,11282916279459081208, 7451328679343209506, 1642167524381777110])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7451714777185412468,16375762419516120620, 13219419040235365883, 1984244295999786739])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1678726917412427052,16681691301830811427, 12212311599187224914, 2788097775450328999])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2581012450297507583,10222133311161623128, 799282123690971231, 2596140266053363343])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_14() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17467116737658134753,2962194606754979469, 17562249865046423660, 1558582433047439185])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12485279394115028284,3093857286048308381, 7668875693926280991, 2896918336711051822])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13684202291829132745,11955797385007293688, 18116746553950579338, 2358096294419607240])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11041963097654625501,5773205162823380177, 18322427216244822419, 2719537810844851382])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7696029290109932072,8727119041609287685, 7520859639868985773, 1912557522158310588])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7942651865690727844,4225587886154400746, 15594960042116442440, 2884253335520974473])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_15() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13771253454421949046,4372329653008845027, 13490096643465732411, 3283088325524048106])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([780051434056683279,7003474700794809006, 3081067483400157499, 2531493385778591460])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11095742197760796149,15352693975838897542, 5465781615550555465, 534195878933171634])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16217998554843648632,4147202279263617944, 10849469570892518166, 413171043929178686])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5429455519941622455,2260030212451794498, 934222187830417519, 1542780013171370335])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12311737490816567187,4204185684525652797, 5402785428767514681, 421863487067260537])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_16() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4674641429450364934,1782314234706967495, 10182562054480322302, 2700484470331142055])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17149179863271908199,13009890682673618951, 14410858168681441670, 1297980889991541792])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10055146465118948691,13501106283483168874, 2930870977440231787, 1850508168175141205])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15734139381180524721,1654321953347781755, 5140918166909739446, 2805435461073096545])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6711152975053078941,15474625194775268261, 9828331908874558975, 507515009127940965])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6233386289315440175,929199566310030341, 4070771682149273330, 950108595634307904])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_17() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8687533107582977389,11157379580203897316, 17164930678835406942, 445331002843835063])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12694717277725370688,16575054188839352824, 1364381946756106630, 783155829683560551])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7547848914997326548,921762229277869740, 16698825710494424255, 825979163766532712])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15054323333728335493,3187727339680134638, 3459000851940121250, 2803197912290513009])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1330975527379074847,1867711366497367523, 15134235907135449639, 1859402484424351907])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16793739962088395611,2759875421771830060, 8093204454538317878, 768067231958304895])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_18() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17101690817414942904,18115181117169256069, 11807000760856960126, 2827618776231711802])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8570150613949432060,16376385786124759719, 1413449817394694131, 1615238984385104739])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13653155821748986010,13190176790511453760, 18299196884509063367, 2538910935053464205])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14321624773628405468,12623492480281889549, 14563545512873150095, 2429079590505515360])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2702858545940882397,8766940920735106273, 13658182330043259186, 955540619263867492])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15363330278651887733,16253727388259053097, 17087744214461623812, 2881834931961438406])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_19() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5238576504548492339,13569744493088737366, 4425920705234366575, 1830167624928101358])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15395358067611193149,16991750855702635746, 1104608559804993067, 882627299019218634])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12273409135836699808,12330950377775584762, 9483146745299702571, 1530069412664789134])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2681490981107688748,8496457591887494829, 11312641574440021008, 2564536085731753069])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15582312485022177762,1242310029392200598, 14708207126984001905, 1015045372719682891])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([479454282847671702,8645289395697181912, 5656864897288939811, 1611246671104519401])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_20() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13270338237339924267,2432333242422626037, 6764965595774905854, 525164810766887736])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16615185634026131173,3746083490363747152, 4119252458877094499, 3453172701987797579])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3314280391267778690,1140100331474692052, 14640096144099548254, 1117517913331461360])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11445429292133050135,8390297022869040813, 12606574786097393216, 2996544168424426592])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16591018523678575575,18020975724682530455, 479543402509529841, 1353561245736833998])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13477844187568837439,17542531435638815044, 4910560491646963780, 2383364951501640434])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_21() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9425260494631648991,12543157831408083214, 6210078458092491313, 2047348905687436846])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11941492372928643992,17980568730669234918, 12673159690586288692, 2869932410465208584])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11805461519855996676,3659075228876078372, 3503057885411584932, 3075365451547165116])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1759422301121024472,13022531159753570072, 10689810033900546704, 2094083329480684569])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17823109643049523704,14477140441704947151, 16722876910166673300, 1353305876004310894])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11251011742927422634,13797167773626476541, 6092779821531260756, 682114135993464378])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_22() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3775184335726030243,10268861628115199886, 8706975839184966490, 2438776358104632205])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([487909026553749089,16055656004556544283, 7838440044961328948, 202745492506594528])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18091049138865381733,14796690530740098493, 5504417798397029533, 1091067233161557430])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13013522770067998252,5824480758700520687, 988499486725541000, 2590494095488203117])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7138372299195772490,13380183770288934550, 13210057632571680287, 1120204429641002189])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1066696263533859803,4405520000113722673, 15909853645533168814, 2421270274220513841])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_23() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16579871912455282766,13667052762180540085, 10248776272898010406, 1472397065396912070])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9110187717655578824,8917702343809265968, 14494220151333566192, 2987330196043648150])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1353388341767340474,2072153925617039038, 8405072096232500354, 443867344754938364])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4920326628026994625,1521686167171209052, 16478883870533995407, 108672977134191488])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11992931654830474630,15556447223158749379, 7128420329750610775, 1470802427742585053])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4918738520878446389,6637405872658072248, 11400593656285885232, 1386823885356426690])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_24() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7430549504838691390,18192179645183749088, 11742310497759315455, 1004478350785273390])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2782507847476585205,518026636718131935, 17763293222521936548, 823197396778279855])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8382434936813517171,10675205106706586279, 6498905741231283371, 2288116924714018990])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6918730156100146135,2590566195401079223, 3732882810323968667, 17766894922427727])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16759305814250543252,1433933912444543642, 8860504768030314485, 39424136076780217])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10006964533727421002,11487044809092799896, 10700074899820560422, 2271175300835428947])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_25() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4272275979458605855,3719004736571034085, 5283912549760980014, 1653528320437157769])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9093637444557780592,15617309684849202256, 6491059365724728586, 3210824504514442765])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8001115053955837840,3661456640728388116, 10092750485646300593, 3223943296946283221])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15489357921379981567,16757488858936204024, 4830881066407528129, 662554535225096084])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8392154586548130582,18022771931539618850, 3555459866948045235, 254280900213107527])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6962148263949206055,17029287761709530817, 3356313480743501818, 1653141656167723987])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_26() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16786282327014074858,4216840433020921920, 12905925123283457557, 3260165313885615356])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13742649460400515817,14431289800316519106, 17348154828536986172, 2630157310753640929])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6695672662344060440,3195955743122731602, 2429885277560673635, 2548412854032608759])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2218128652554015342,4964256662881911003, 3060007509241814379, 42970251407342802])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3568510921578759449,16102730717266414644, 2076347379152083619, 162967492118794271])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8290577809316681554,13852208608456445626, 7207720561625933517, 1155057704458521728])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_27() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4877287086772820855,4939461123237901222, 3175467933626788763, 2156487211202258977])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2656621092674354300,15450778467665615896, 18393431673079856902, 1394241678905927443])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14553974652719878168,7499172670186433719, 11488770927092659971, 274501861702100950])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3522966207942254459,17790695358159403349, 16131715198145289353, 3451820803595789768])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12233195836751259304,8650220966870678344, 13625879338702161814, 32400558056429538])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18119821053986617081,12836668868803352117, 722215331888219295, 1311573676637045551])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_28() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4536371612829441387,18167056602364044269, 17606531019091696454, 2144728179033268137])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8995316515296774642,8164417025358240014, 16866713073442236004, 1284009590614934708])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5007380702462376308,2717153154101984315, 7894536097315245871, 2025591017538241981])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13925746626874380860,7476985988618865877, 7196849397418736980, 3059753842755984896])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5748971594887906745,11274350115200103783, 14394582410788126769, 2786458870137114320])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13196632088873723237,8222146827445959456, 712637615416654057, 644118266373596760])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_29() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7965285715133958495,17079142206624875102, 7133000081442112536, 1532875861227881781])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1215993684601409929,10577589993382108947, 12998617812172598245, 2809795153868516409])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9371422923607131121,10430156056535134162, 16078669151912593155, 1179054010550630508])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8899064082663998287,9196489923561025068, 15286931289091001540, 810877617348354025])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2999578503991958778,9105437269810790473, 16654781499816882954, 1021810569551212034])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6284549018294116578,17171141556066672692, 10815306206834605150, 225976863999044692])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_30() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18275078107546919438,17259004499882537644, 8055513133227553574, 2887575280268958523])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13225690647966618903,10485550165377989187, 12060107325146727270, 2802616764656980857])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12553023905965798663,16856880473661584621, 4603886629627308007, 1260301930216028089])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12759566779383361183,5879963232135629277, 4389655742870267766, 2951242330736994944])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4805316845326452226,15549879907488219693, 2748558519540688483, 270556304116560161])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9202018620322260730,5743469192756575698, 6370540404149883687, 1884191189054489542])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_31() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1965744783601545108,8519194184239411257, 14920999296794267527, 1061442746676176800])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5755095094625201985,17981223463803300407, 2715979444955373209, 445121758505731588])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9975392504859966100,4528815079807080833, 17683840286959269783, 1124301200936891583])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12877934223849788996,1483966859871349871, 18296121996679504614, 2944791574046475753])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5521117708168761751,3886255903908424130, 4221871280528073829, 192597837467693225])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9533438861542075606,11337766222073302233, 8603704744460762088, 2308130479891615521])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_32() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9127127297733976020,16172661129682131386, 14987095076192205145, 849601678518761575])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7922333719398678921,5159772289657963877, 15384659818728346334, 1252657335665007016])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16001915281102333981,5057727671627379082, 15858317345897571518, 3292119177024346299])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6452904913273388604,14984602450754041017, 17913745750469240679, 2968554790078166307])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([397849741945792209,17909386817979393874, 5927596327759867578, 2691390715548625515])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6931362051444205975,1665162105766036303, 15782557776159686426, 1997420091766158048])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_33() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15861539264094778008,13159684766793959853, 9446683529448825752, 898918435304309591])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15663769798035424746,7884210583380565259, 11130054708396950441, 408902401639935375])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7802405724802843755,6434962270681824672, 8513672443901111886, 3471349193876270978])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11515272998916637854,2620837284943215751, 4803103847054113153, 829923129885020480])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([429663099935328535,13547220512511096343, 8442360954379435875, 2120406608072554873])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15611631805674971364,2709891120160713159, 2012346116559972467, 3160782895604167285])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_34() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3819504484088508491,16364655006330801419, 14371502653384427244, 3158935850947299545])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4364374077290555295,9960346724395956570, 13737365596354094623, 2183178634511481045])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5507838069916331855,4324417249968007124, 8289609284057814791, 1258955027016666386])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18008986303543771738,8505576554720674102, 14599172116620678494, 2341776787181695223])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3580452335387014513,10979932493958869716, 6258530298846009374, 2696255115712164364])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4097949756031772425,14040185060084287881, 11780319761490688361, 2717268145937606682])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_35() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10290578917791655416,1741926456338088913, 9138259700885133945, 1264257080156282058])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5803434631555762022,229407647237523353, 6111974471023345080, 2784885823218707663])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9650520010477221396,15303317704201830084, 4833327034600992504, 1508743936788130448])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10181531788914354176,15517076372839040424, 2976509160942027047, 2349227261662365913])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8746331965996236587,316182562753968814, 11852761350295948614, 1449304217339749644])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6057435729666248114,6519708073755513302, 2042026743397642859, 287788015319414778])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_36() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5902941089074343759,2980213158688756668, 7291898390704312673, 2538894975137000527])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([478726087934854706,5876855093600094557, 10666002873851772995, 3132925974631415352])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2121550450419404445,5987889763410724127, 18196954007751738092, 618253228184482840])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4051077117020040351,869153733765848846, 15247879762790207251, 2969216043479516156])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12545668870467933462,9996011898278412170, 4903156819683450613, 326673650875809076])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5710488480341631732,17865292248644429247, 3006378986043752957, 3189602244296471226])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_37() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5293873799129840402,1043789393342712268, 5023819467505025130, 3013498498475335136])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10108860247184424566,11940783280121150159, 7836568725351756728, 3139264591955809765])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6548968291335470211,124496487970899219, 2685672962136590385, 3305283886289479047])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8366650822595340727,9782566143807906637, 3617760048926118828, 252113771081657275])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15771736370832419977,11534694787134192867, 6136140057594617867, 2035970390419566387])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16042456951134024957,3446118036033287982, 8010641970789919637, 2443515396318547784])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_38() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3977873480332502710,16699290043296547975, 14911634284622609530, 343917938446097960])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9458646841722698233,6017670783013612573, 17751734424124872293, 1887333379519634479])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5288937219526263045,7713822688434553337, 17722376342119500598, 1714186968213546743])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7978712335660646419,10842094746951655403, 452646066315069155, 3305241883357566752])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9450096657475285654,16664459400762365857, 1002029222492997321, 1626335224899595532])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16826770018122032320,3283068113256843230, 13338813553865624797, 1533019063912260770])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_39() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1642048571513915537,1323156418835581005, 4941278048615956329, 944177372244220026])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3007976773772562478,2424657596983033780, 2975802775208601346, 1630182535949330050])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1754248683648433617,11616886664463961055, 13177183675666048757, 2137165440167455236])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17935734731318019758,17769878536127203969, 2950802465589499876, 3183328463665006019])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13947139569544582258,6841396392749252470, 15003985434745551798, 825143628429210091])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7530562837807810880,16124177939506206068, 13935982814819742738, 683765440008515560])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_40() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2286276484988642897,16258429974199545495, 14044646801604564022, 2555370650793412462])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11065881892307195499,17239161327008669051, 10117566779300395139, 3193027968127502950])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18179506429115127328,7471312019872153660, 10212309161346832628, 1787132679658588531])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7999408565751939658,8396657341694847911, 3903511312848375051, 2083324866239529972])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17399086971505622889,2921585319909741800, 8580897363012280158, 1867330038862329396])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3594880006577463775,6766400523249087427, 7157092386399797260, 107932077508997984])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_41() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16835780258210346369,8312075508176888848, 6632076705781773629, 788501903927783005])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14922243790483258523,1864751153031777290, 6628810789894995900, 1965135679306646644])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11345130697318847732,17065117900159223011, 7641807320691863119, 3470219482391544306])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16890303576455058513,17949551959817727604, 15853736041666321389, 2478333765806292322])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1980309633406277172,4002231095764697280, 2947744100524615742, 2800230451274390250])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18395765998511887215,11012581015091477293, 13161996951418058995, 1189534847289259203])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_42() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([331558633831954124,16279297590787904750, 7117297020041155672, 2141245055803048198])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15671506182462003242,12032635318905203610, 6663360623857009940, 2285618947749624410])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10299269739380875653,13424943419691995521, 18411249890633858376, 1708730581008398665])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3373025613625586155,11696575666541682932, 853977576327547948, 522448117044808683])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16351824443115672431,1138867682783504771, 8236269581869072824, 1283017433784719252])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12868721510388289767,12801673815585307440, 8089956267688322896, 2102876468276826038])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_43() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10616694366865720839,4428173365440723396, 10672215843888791151, 2274562247020991573])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4441692659240484149,1973478806000183071, 6027935060606481984, 2809719320564294994])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14954525876098837778,14790642254487252093, 18065318959351251956, 2195597625250813914])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12328503061566388841,6245469272102904511, 1787627287583647626, 2774585724236006442])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11938826730583676505,15399819644777322229, 957995978855803699, 674025096331520388])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4973341880099968371,1681315934565462508, 10040551858842760193, 326410317506670809])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_44() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1096216198538343049,17277007505222377153, 2277613635925063389, 2712586873827809768])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5642427436440579058,3876632697030657188, 8108295779289738516, 203662470882829537])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13540114990911189308,16845111135088461192, 736872506501311668, 1288957236624550872])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5981282952131986121,12442706503874807622, 14303283968192777942, 2630117798742083245])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16436336580577102950,10175744781336600617, 4941998317365404803, 2495177432406276115])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13594754604463109369,451735056658197036, 11071313934488845767, 1449231019102279601])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_45() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17496918147753282686,1496141978525009388, 199472598671738030, 3061177075559786091])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10084980072803565654,7329325945233990498, 17644730612380221152, 514973361955584768])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10294194766242682260,17971198094676147803, 8988840253817377535, 727811514977672322])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11683312167831229300,16794605036372880660, 14870866445813359835, 15186104806502721])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13239399210283796234,2862078965960299748, 13725854196429729988, 503913994915701468])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2744395900322447889,7628629910938572115, 13985206682797267864, 314782240378036003])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_46() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13196019140230266347,16616541137995901480, 8065732180567333572, 1798992218193123808])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2930045519104717535,219977813221616537, 10607837391197711014, 1796559221486981257])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16965769981594137161,4689408922730149802, 9335217917796268917, 1085279966229450515])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14068807599632934508,15103639383845956690, 7327183000686969058, 3062036553290635781])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17777975632826642097,6165477816128393137, 3820833767285583828, 2475726396100270857])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16493357675870313224,17740542817607755041, 3867574678318460441, 250284145421201912])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_47() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17454427590095517611,5605107691638440044, 16503936519526034902, 247762625775303799])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15919077861818826621,11808221327392752815, 17601357310688758929, 1392585370396525050])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3901464431207748728,2154798489473740385, 17196716852697647360, 581021861318929261])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([278610426783713533,15675025896900250313, 4286275864531797417, 466875320399742270])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3001346743157133065,11304217375283959478, 1742278297723371414, 3152191295369894750])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2663001507035127183,14171465720445110098, 15285738302465225552, 1753864443128757929])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_48() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10706966681278427041,11761559705158756439, 9652025725658744831, 2466501492657985868])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16854418537712219621,8389943602109292569, 11333296736240370740, 532634541863391383])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17614291390782214716,8409077678746119358, 48196308793229441, 2013748936253018356])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6358685409495095210,17142642434087902339, 28997665759973704, 2139819707380563560])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4423741970423684576,14479716052695440116, 15695532818980396303, 535664792772512480])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18419286796470066780,11839070851029886632, 4177612866099811662, 777000035626294719])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_49() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4727968930493466881,7418967508030713411, 16564078502008537200, 2924342084682300462])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15864470849437486435,12460878797817467903, 4932277332099072537, 1102669622934907993])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6321322925812963432,47007879310646229, 8805500137206583358, 524441455951062302])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12983925497221976904,4229559375074370653, 1850035157478081863, 221849244481397807])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14858105529150016237,2051914762407353692, 11101318165824126957, 2759341131918463416])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11719544956609922933,9978385196678702373, 4999299037557413320, 157556529568080337])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_50() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9818266814887680622,12958757838748673900, 1189513031146628952, 2230981373165041760])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15140556164109645175,7204863104836697025, 7540963776304673564, 710048138971973244])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8735214601890402018,12630502866508941611, 5123086664092058879, 3017648117307528595])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9780279287604570593,1861895347367054158, 3536219783841812755, 1205947698083919349])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8507867470874878832,564248279600967936, 4361355090330779470, 2624310498457126326])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([702852832446581513,16877048761820136017, 15571540799864560094, 261251849366982628])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_51() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17184259984580790065,347934259421690621, 233011436993962185, 170878513933027183])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14593707730700448086,6512772460115472379, 7845595424493991479, 1715090328267418959])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13816233043649553875,3438504537147473509, 11913674291265263477, 1167522649591728614])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10425407157313419956,16195128235820262391, 14253937999548037165, 2482083501203193925])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11562589162160093368,865899255460842783, 11490480819969084523, 2828045249899551547])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([103510343437759545,5122592784873494826, 8075134996661574221, 2855061481131794527])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_52() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3838135964519270291,4764443701836271218, 11470397317492882444, 1157639125737301416])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4187281954831186618,12398735489973186799, 9309672840191997869, 1647752199535785274])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6284483009737965592,12483997968509236179, 845659810256554450, 2895867243751704032])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14938808205821137038,167872831094018422, 456002428865227996, 336122612336845409])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([961395572838653694,18012129316910801497, 2723406800604446481, 2406126803018199148])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13153226916794815187,15775432034753947865, 4633853352893539016, 2268556331886209770])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_53() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14658560901992272042,6226064950556592341, 17246756083690562015, 3343492585525801169])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6698562456404075869,15226271505587459618, 16373231284494131712, 1299590931087808153])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([366990310477079793,4953161657587899628, 6461591013769318015, 2535469478916399606])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14564474586839386472,4367362029010583348, 17822795371331552575, 111385016173046561])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16116367088362688195,16631286116725932775, 5358389583589270334, 567896372087379078])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15990460143813255060,2451136005143900228, 1462287540185845451, 2808359928830683037])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_54() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9764162276785401652,3258863141872455049, 15049656731871710505, 1973703040444455185])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15456806113099831629,13089590749499716031, 152390385725982706, 2495757727589311630])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15354540791380046088,6251782272086483585, 18065021001143320916, 2817220500295836551])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7386696690803099833,2917511568818362831, 18196457795189105061, 1176580811960639130])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14968817182989336554,3907095760584897585, 3859264605669596073, 1577151035773576844])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10710277361873147495,16567137906029287372, 13149195320764534600, 2040855142784863622])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_55() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3012557873831942115,8862622321179115513, 13438188033033101152, 319698322551877172])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15957772820658177627,17898161097559726414, 11512190321647315700, 1296086440361631052])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1248662647031124461,5522728054156231182, 11265804686570613551, 3122058209164870999])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9218637510147766670,2564619054238868534, 3956891085400325039, 914777118208075383])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8828778080364011494,6683395095834480294, 14986138208205865949, 1597661673157483605])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2038601324103755240,5745590517678701160, 1173948732869403851, 1791712269148429679])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_56() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([401292700389815394,15698137813832159814, 2382085132053859721, 3347619628924325702])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14194568129316699287,2130864452828124858, 5919655517432127083, 3471903008307988427])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15737586034462839397,14655043014362798757, 11170658557512470591, 2933857535317237511])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8807246528598188438,14024663933483691727, 14828844034516530301, 3423881229435064556])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3156299521751155402,17882782050501753279, 17921404878930908635, 963100584997896305])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7041852183602172424,6961496497417145091, 11485850339302772065, 682777144095551626])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_57() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12503449468046385980,1306620803081825837, 7990524377696280409, 2124469826008809088])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3834472849872594150,6930021728458131762, 5753398271764195482, 2696439661646213907])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6063109797661964189,1356197103139578746, 10798670942021053328, 2418896484890899576])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11417911816242672461,11870106983462516553, 3246507625939561208, 2685837460238674211])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11095958587142202595,8254159927697567833, 8704046402250036682, 1506480113327510061])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16792201955886068646,16713211317709889117, 962676767895847963, 339443610496148240])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_58() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1043153034857546597,11651075265010742997, 4437214465708679754, 2470223265676486080])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7344023555758646777,15987275856664100561, 5686579984813288938, 1348159692585702430])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16213110713240159602,8128187456490203392, 7683559636905641295, 2385963607967657559])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13263661038218402748,10175519210249757352, 2477039877116426524, 1448187515822250611])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17578228983873910432,17086328694189885679, 8145422088125818066, 1904705474159399372])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9378697021893577787,10604723568237626379, 1002073187913498505, 3156804838987374798])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_59() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6911909076113888409,10766953812262870391, 1095687407694453514, 3337480391330577813])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9776673974344948599,784254186536614914, 13534708332697664272, 2092522578825453325])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11779666996991097418,9614633499590159043, 10734994278186497770, 3206786385070411402])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([585503782022634295,7319964381224402842, 7303784232754351608, 11040208987622542])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3479946677569641640,7479618578424045940, 13645542416101587356, 1664274598440493320])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8227818948632430233,18272100610536202392, 12504630277934148170, 671711940673581092])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_60() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16377342602797175402,5887408968015468220, 13872913694064505095, 2175208062595378289])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6767018068377901089,3454126684320906273, 17089570988301465705, 1680545804873529066])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4815298211769180303,3109034195583884877, 17355186372301261681, 663058300843791813])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([929097049931795760,1268634976005166139, 6283666071963105232, 2759853663400231232])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11665541773961169538,12359718613418761215, 16771322638473814597, 3381381229416742287])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1818898064419710888,6614942074074548377, 10062534390082888112, 2979768651252059735])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_61() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4556512468643841149,13174485291737969151, 14505930984929111543, 2054875745703624246])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6319221426246439544,11581607425246769458, 6303295983543594862, 2609124013812872552])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11111454516982028732,10685202286559354517, 5983059382285986266, 3322427119116258859])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16875203732306030950,8588072881284423680, 11679840781330806683, 2699272163884859689])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6200412104961471645,9386947396573426843, 8338690087547342671, 1079196667592163891])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14733561720955715029,8783553905629724148, 12735248511150075516, 1983333335389049234])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_62() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10926110562080751590,13695330723526617034, 4787015972189603469, 1481969534841158446])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8752186164121801774,84867776772877895, 11626455351077794561, 363867967908360567])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11268952397879569044,4409710664870054007, 13198429269440574421, 2292249510233183980])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2535132790307300936,10336180936454539642, 13896402906338427200, 3183917317654165236])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([179299963088879273,11723192443846049050, 16950695389532707715, 2998688332972066230])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6847188452291573196,30062914769667799, 2834309197259827585, 2764929447375581230])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_63() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2142421604214690670,2998859302289551117, 4976861107138457723, 465922095011273984])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8669417569979986435,7709394508504348960, 969571946354428851, 1147415717234176568])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3681005682897125593,7456173809461700, 14083415586976635379, 101634801129658753])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13622725397282741000,18154309822036784241, 10409271307329997683, 1756953116505024305])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1130446032848885753,13066254331609761606, 16469816105791070745, 2092129587900511687])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9795872142795090160,7898796713237299059, 2472032153330124262, 1317710115536503057])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_64() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4126745965653853857,1783374307811828363, 18417997148578741909, 1971927271458927357])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16515013834362056679,11098943256202503317, 987042167926384958, 306408027315471610])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11664099594281981332,3932640419535175316, 212196725269560534, 986535478372523164])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4453124658834884187,18930194340558755, 17818412198367887504, 159239242363974153])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15633315163539998105,2491317537858664699, 2762971259357588548, 928281645202382336])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10134361470402345427,11475226708328051811, 13928737135402303908, 391659789877202545])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_65() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14940514488037453054,7688305972091993570, 15851627055715386337, 1799012645464975708])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1210002506876269172,11600039562111311275, 16986638305510448502, 3325956330570683654])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9320110929968871791,7159234776493663826, 6056634440071672650, 3335574731875489369])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8691394102149358303,5470373984601671412, 8326347114568012477, 1903585304562512712])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8901991485881500937,16323434252300378853, 18433184794030331326, 224540595662988861])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17419821222538212122,14620517711379040416, 13348914451580718620, 2249257911100639877])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_66() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([172475847887272106,15620211103010920188, 11368783803174217131, 1818495068889416901])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([337090278061067904,9257307957901683197, 11297265224384330574, 1130228583361241014])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5097543017767741222,16223331294429315702, 2395847902081452724, 2743923758814314064])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8871322002296578967,10758405044089381610, 6165898155970529579, 2896533013272096130])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10309011567436318699,6423354979071555478, 13130614652886095613, 1318635944767364753])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8441720805511572015,3254903974081251676, 13669387418687579232, 3389234141662860989])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_67() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7412683607522961825,14006515145228807621, 13928065514850217436, 3207282629278047334])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12895769427097397096,16681436888917886193, 9693484276076638290, 1696167671096747794])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1871917212490076754,14791552523697032045, 5687071219005459270, 1517029311710534212])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6992970760461476809,18395198966103917598, 16469371679292454971, 1576608417624308908])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2769152634419224642,10818280646366361267, 10277047298281651628, 2697200591323347700])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10222313256380061359,9248244670523234235, 4846181705678000673, 1665342746800958187])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_68() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11887096097189815595,8026581009765840224, 5737031141907586657, 2708561989314937768])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16569355153919454922,15751871329380972306, 1275877449886087067, 2303327107003400945])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13537219286988921974,5185203329994824784, 13043904763616188242, 3100374552579098553])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([563005552015239793,6592941869465819939, 3852024248264272421, 2406239530514325429])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4544987014374590716,5948174794655621220, 3155780405101328422, 2301593672803774600])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([990801396095580146,5738765630554159360, 1624722394628523798, 601110948445052243])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_69() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2464162719228617047,9050727105007649857, 11224929047237451944, 1843134127468486830])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8207195715477219759,3535265398588313143, 15186373054291991703, 552854027347991489])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4591701035955701060,10203334165143404220, 13831876858124173850, 597766812159744053])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16001870564582376926,11960584197198053233, 9859554483165949923, 811724089857420425])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9941743600525113553,13224043622583743682, 1680593331378813350, 141995415203939652])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8364408972613007732,13654476501767706065, 16363261677391558526, 3082034180152118262])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_70() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10786172532089965544,17197189942661605485, 229878003143007984, 226263492698303455])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2406698464316140152,4810839589352939722, 197241458462094451, 3100388825464452792])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6585438397786710069,12718918836959120054, 6189769253020735560, 776507144322690833])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8185737717877727357,9629189767725089213, 16549651474342908088, 1463553421434811987])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17485873010393239183,17566254299065375853, 11093049160111011792, 1606939323322483708])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1274539220996483879,16439204209188735120, 9800599061950658191, 2563945520277413322])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_71() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2346714202066291486,3207754480326260194, 16975665259759092000, 2090105442606223504])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10162275465450217746,8813338245280596673, 3089000816763950959, 2064565835480819895])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8495452752662294042,18098514174221907626, 16057759062604505516, 2311185325476633384])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6759628915856818635,4750985237304138555, 3068391364092126778, 2746395514304327967])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8006039343453382679,4180027064983073137, 8300251638145888451, 74041454942552672])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16713914060915331524,17242437080468950239, 7574696559873558769, 1679858588834232828])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_72() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14524382565146033254,10281959046068340793, 5861848345234619740, 2417090280422382161])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11708535420866694357,9016996886075561450, 16950360641326341328, 770601317258099842])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4650343130317023291,13722859776059841099, 14813260302070842727, 693774527593618038])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3177146292337295766,7328284421983539569, 4402687369592639558, 2715155436923800420])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2520824780995847623,14663407873905636939, 3372192891290572250, 1240329133405549480])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4972666941305359272,11600630150308068142, 582147906790547239, 1237991907304430228])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_73() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3356841205662439857,11817509741740834444, 11176815830633218955, 703587581860581359])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([610318584190961841,5128251347161693745, 7727171894295288111, 153709127855601760])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18331369357893699136,2759306160270692422, 13468781799059691328, 2421360669062513835])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1933923029525669121,1032062646849761300, 16333715515891196572, 1955646709678117604])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4586663787479450441,8632883211191961345, 6141153471593413231, 579199217154750287])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([775156664748181514,15181717753538269301, 15210388976705710055, 1146381198256225197])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_74() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2033550854376695378,7587907370276313146, 17755127431845529875, 85089924254726093])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9994528862285910292,11099246557416392009, 14927932045086109338, 1329626832549823847])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([208589188391847158,11761487061751220089, 10748510218598392838, 2534050945825788519])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5342663019298184106,81629380192935030, 5180326024013340247, 2986738247736300471])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9387644756852604856,3771004004760087221, 17800917353279371073, 3429721171153089444])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6610246110738768331,4582740349448744414, 6468215950519099871, 131568204566230226])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_75() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5256173784630589084,6647660920095572252, 1312072484927315355, 854780455116797212])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8953600835170780663,2773258884281564335, 13665100602127635010, 403284915740226677])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3009570850381675225,3708717964605528046, 3045417872596567175, 85209451173465498])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16402969106779466553,7372353737863236845, 11367295722537976761, 736638019574224233])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8123550208512296389,7324698492713626779, 13592010230668141672, 1436190037219863477])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17099818330032951145,15959413728482738801, 12729355339160847804, 2885419828653836666])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_76() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8661041889708442979,7575407849639897693, 9059256600647745980, 2188872321824398020])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2156174852590255786,10013996440115606332, 10296048010179821634, 3066313295455155496])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12433295851328783930,10071376464610042000, 18115002189858143941, 3436086085026540001])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6114147187698004355,13570189873489087853, 4923006950152668018, 449398401435605434])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4675744249074247588,12380808881488631132, 12839138695041001823, 1377514496484168658])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14408034264352963655,8059834261149210074, 13881868486416435541, 113240787965416451])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_77() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5109265921808477908,13292334906679905478, 13558340560034172389, 1477274147700333613])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13554708950187554437,7309216853783630526, 16104862878040061426, 3135707878405973801])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17220756732687911174,3099178533813178724, 7980206270351882492, 3467262238594447080])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15301393981077708225,4517730843097722612, 5215457477299472712, 154709874339877511])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11626811204747895098,735304637677405196, 3360786477130454648, 141766116419486351])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17130270706751014925,11558683137802217968, 9446178508477065864, 2975289109698617337])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_78() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10907224060160543151,13709416154976348388, 2086255074016870282, 2062540824286047935])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11284737963057987831,16734578451186506067, 9197504276455860581, 1397280878556689751])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7999481607422010721,13914045520054544619, 15567436174126256402, 1179544700240641481])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9196416475034021687,773158749659185028, 16338517405232099686, 2998810981589126912])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7216191852498046655,8618940074417650241, 16010240451553763912, 1061194592807095057])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13301829432147972416,7616355942368019813, 3007759312560317981, 330608735171482550])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_79() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16567184008621128889,16350936527689219522, 3180911708219195825, 715308030451699407])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17892161118065929068,4951619638057115495, 349323437779052671, 1330093298881202600])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1532738002576788785,14406245129898878723, 3116066205867908478, 1817246142645665715])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14512954838825395412,14870331888562171878, 9022475040585498308, 1741119306229835328])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3227900551988319170,15294283249764415624, 4298493333870063078, 2096133942168634218])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8230872590323728167,3541498053135857312, 4621750507524893445, 955531405268148819])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_80() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10796980848924482507,17537237776508009779, 985091849560642338, 1407881876112215689])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9086365652634990272,15362573657216482080, 10950011524839040170, 22213423814530085])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7566133183696148077,16278767908941637251, 11283018279893781576, 3023356980240838084])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10596824022036955322,4830121821128099025, 1728153959892153918, 2520923020317605456])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12523175696385821799,2445149148749713750, 13556524819191950535, 257772922265463500])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10736122936555993087,11587179669324982947, 10244024435896100014, 2790295126712780417])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_81() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5215410332419333757,11056702444807943435, 8645672499281477077, 388080829511605090])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1384253489006281588,12477641460924641990, 10667515096209949243, 930197010865386415])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([35432772149860794,1794498253973097432, 12072287207415246368, 2316691174598425440])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13981978932084951611,13186034255616783168, 11201576194159255879, 2599087422987331991])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6945764407658418293,90376660789624742, 1383101905222014108, 3266053143897245504])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10437518080772050981,8918781674824311698, 16605788450964369687, 1712649669227004606])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_82() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13552186940030496890,10329390403222498846, 8879588078244478765, 3303968505541812878])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6102763960093097326,9978756435798435428, 6771723152306752273, 778932018225639372])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12276438050665687894,13745485439580221898, 16210616014669550250, 2819369894288414560])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([360264755871250817,5788546764790172830, 17623315976956819313, 68584737840268532])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15739988999396446565,14468930480295201015, 10122792962281014056, 1199306869979296778])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([684169720228973283,16870249443738243419, 2306386897282606775, 3029708867596336145])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_83() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2186485303369283546,7280118104449904524, 12384304036728966294, 2764539158428751051])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17266259935345881436,14064785326328324772, 12153225318016645587, 233774966404889103])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2276870203175539287,14799451709172495786, 7935659348861709860, 2910752512019147545])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3975295007837850038,12181493594891742836, 18190596815932008851, 1838279263403482619])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1160101475549119111,11521374481206405945, 3928077358995278133, 868972643021669246])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1914868531131005710,10601854459657037080, 13720748004724435818, 3361191087912895491])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_84() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12966794593736657683,16386224996549664252, 544009871925978900, 730205432229011019])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7423331041304864730,16296139356989813496, 9768398152901762216, 1508390963679330473])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17583180690183759866,8585883291411889094, 10527354851567345643, 3437607522527377819])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([726347899833262243,9907632214582505055, 9834662516332362547, 2368570709434893809])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7980638141179390634,8950410160133097911, 40484218525279384, 3341708589149304179])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9070114294241877310,3829810314156865840, 2464887728185028462, 1036037757641882559])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_85() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8889674157577900187,5891048967816692966, 5852829986171996961, 1540567430789897417])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4139839482961588105,16523450607632557355, 7447103599477759061, 1171327884580678825])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15863696646466366371,5926059331362354307, 12856122842380635277, 1507778370669715508])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5569495897714864080,12544525527206501674, 9865848104386377874, 3304635008152551585])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13280361826827247437,3734702863490660214, 889337731180342529, 1191788745046598526])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5099044988089989881,11122014325713494530, 3617506134459083616, 611053499651829698])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_86() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10973819471769502434,13038122664537831147, 17134038028452606615, 2237840042197845591])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([319208828941902920,9964389687002494390, 9713240749260313687, 3280509707194980176])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7853935443885362142,5226581705143408934, 16008565482823371259, 3055638656884645532])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17375893047665980242,15289470096713902400, 1704499149125667177, 2383523615844322793])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9519823624703906583,11264766051649784614, 2683639185923876747, 1430858984208383631])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9310665407181006387,4683789740518508176, 5983487021851136817, 3422980797499148051])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_87() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17336806070465554680,6246775486149491495, 13798017656271998996, 1058753142417862580])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6753293160166774925,6754555294717144963, 13742396438520830102, 380367052922719699])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3728841595390064400,16039924345275524869, 11587836443853573013, 1981178845999714977])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9187984416100433786,8590223448638841393, 15209075101147543192, 3017560519676652907])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7404187167042779702,14814040847070332476, 5268992941362027213, 2695067758385729458])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9201779902976972698,630335261673223372, 17130702211148714260, 1292187462430561268])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_88() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14630414725158001006,7201317797454043669, 6860908640046953012, 1758931419424731220])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11186217627916455943,7777400033880286538, 13468168643527873600, 2998130922708424970])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15596858293904815301,2113489233408504921, 17621151947782708461, 2400199983733803353])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12495468233359515911,14637644587916852175, 11255882376145929212, 1695687810129110745])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10445274465043138014,15137003609057380286, 5989555625970610255, 2451427197654001524])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3549803729594970291,3534110387692813294, 4153992510751671903, 627913596620628654])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_89() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10152346606261505893,3997862671209579170, 13297275513599344443, 1619269130035991162])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([148261015467437493,3916198592443285217, 2092320724970524594, 2730311380272114140])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16363433147231760279,2851186654125280943, 11093437060048489434, 1475042961720634584])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12410058108520258691,13934832991465729710, 2948129524328662246, 822930649519990850])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14566358793659435912,8424697832426779547, 421589436424071396, 997074661763186473])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16807647539883908257,3251410262334303079, 15877480827278026260, 449333962222125999])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_90() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7725232336075410110,386261743744234915, 14004297471430729759, 227969992698078821])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([925257862533934857,6198227026513085697, 14688366703760385084, 672580636309939912])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1248333817815811210,184421247833454410, 14026610481668524417, 1564897888602868581])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12694029562138204021,1854773440381278166, 12123347380772286087, 290882149765631688])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4724427033012344122,11565606083441365664, 12436624487716971895, 1521297118747667134])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9817857618600946699,8000884632303657729, 4867751041028580747, 946579022421160124])) 
 		)
 	)
}