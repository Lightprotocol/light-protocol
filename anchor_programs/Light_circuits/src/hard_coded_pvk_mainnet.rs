use ark_ff::biginteger::BigInteger256;
use ark_ff::QuadExtField;


pub fn get_alpha_g1_0() -> ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters> { 
 	ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([129941079445278231,14986904513597369283, 4385962745611939561, 498495035870568143])), 
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3551982070992374558,4387704605030068278, 1260785428361773688, 452138810654549394])), false 
	)
}

pub fn get_beta_g2_0() -> ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters> { 
 	ark_ec::models::bn::g2::G2Affine::<ark_bn254::Parameters>::new(
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15305662871848298464,8730722218528755724, 17655379369929439080, 3094947497961004670])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3734078260244790872,14411564338784214811, 8232620692736535231, 3011377832406123967])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10335342952088637750,8285646875950846157, 11873198117680904740, 1546294508232368602])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1739530176560646196,13381954797402879400, 3267741463057517305, 3081279098028082932])) 
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
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2348816100282106913,11063927249633071998, 9743917121223174145, 744611573766617601])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6864792804873239822,7550214139358307027, 6182122313063643348, 950665323144705918])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6387380389080445914,18137209427592872126, 5307397451549882657, 2871882694267741505])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11513473405384817533,13043232706959217524, 14263719171576953012, 1114943149007782439])) 
		),
		false
	)
}

pub fn get_gamma_abc_g1_0() -> ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters> { 
 	ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8349591479693185880,15279911801216449753, 6217542207949113122, 3223606532813786368])), 
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15445806006182760920,7229343031300834888, 2187096478703783098, 2461903701261642261])), false 
	)
}

pub fn get_gamma_abc_g1_1() -> ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters> { 
 	ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14852821519815018200,16304516249462926920, 13284395724003183391, 3479533145733146148])), 
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([166703934137599959,11117704177676019653, 608224305532820497, 1858612265386931555])), false 
	)
}

pub fn get_gamma_abc_g1_2() -> ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters> { 
 	ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7565621046368407615,13985741898060410043, 13742417189346985449, 2590604941160695740])), 
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5946040741726168311,3766924680600517733, 16383008422921453440, 3228272888860943282])), false 
	)
}

pub fn get_gamma_abc_g1_3() -> ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters> { 
 	ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10071082115068981019,6707093476646222308, 4147126126546048906, 45573237310682480])), 
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7020551970044821835,13257090616643559493, 14700341546199911232, 2200560160852903109])), false 
	)
}

pub fn get_gamma_abc_g1_4() -> ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters> { 
 	ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16433036568085878755,10698113955843378791, 11553572350024152282, 75543333324169790])), 
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17751805098631064165,7793222841694379817, 6033859622706760510, 271205184390358803])), false 
	)
}

pub fn get_gamma_abc_g1_5() -> ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters> { 
 	ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7974856573370744384,14911028943122560978, 6845864340921728418, 870146981347531304])), 
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4029748477177809041,13417257517890318445, 9145400772335112447, 1276374645814960117])), false 
	)
}

pub fn get_gamma_abc_g1_6() -> ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters> { 
 	ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5483440792146658729,18012090736970222267, 15919327120194551220, 477243221285850263])), 
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16471416546110839144,10599899529736790788, 7278776717243068054, 481387542592225701])), false 
	)
}

pub fn get_gamma_abc_g1_7() -> ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters> { 
 	ark_ec::models::bn::g1::G1Affine::<ark_bn254::Parameters>::new(
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15780857241515641371,17304391103859282010, 10498304512185709771, 1637324130980696821])), 
		ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11046941996716844868,3938673765753076632, 7418022171273829668, 1419242565690503630])), false 
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
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8442143906881235565,6910550636998309615, 15780347025534622182, 2256767121732512344])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4580202737060083450,7639721340208883433, 10080694269444354409, 2229886298015564879])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17610375784280403739,15002293980495905663, 18419880882421280409, 589883822600976326])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7758965280668456746,4165979698259840557, 16481681674805643334, 3375885967238841112])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14059656627598989494,383607213394169002, 13446482281871916327, 1510573518791147874])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9572162974670141311,11659756899759732115, 2960012567211195367, 43103083588467551])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_1() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1500430961349725525,15709740573916054613, 13229988842205110281, 1882777262852840409])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4786324756770494158,11032852526054643627, 1576366329843858871, 380382869184125257])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([661159965581843577,9125442476523015315, 6918693887116122152, 2472467943266019565])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12990310295874588290,622411175675207761, 5564814417848844564, 1049658093463502726])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15758812644750385689,263674440786032538, 15207455179231006967, 3072383804844576170])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1103788588999470743,8911682441294033975, 14165202388999213579, 2420869738672567223])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_2() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8636414572425151543,6759138893188658810, 18196761961549425189, 1499254763954777163])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9230390510567093523,3937771761686645161, 14723547842625390715, 1431528548209332138])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5659268306483069243,16935392093746009569, 6352811052318485294, 2672537236660171772])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10598498348809675670,7661977085369514831, 356680282490705357, 537704501900250031])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14178322697069395319,838986791548396653, 13533699051376227053, 2377365610342494933])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13990680910511468023,4207238384703314636, 15270701491478692808, 2338823244948230690])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_3() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12507616331400010005,630873702801280337, 17435544851795405646, 1677004878940551958])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12320929293717043063,8129980250402522893, 9630321978117073175, 2328910636032866955])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14628719589457688549,14244335120133273980, 6582649284548144323, 934398668974652219])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8209340711275194604,1343331533685338049, 5788029133911849939, 2532891705343001488])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11368551522946172118,9659223377928999805, 2953369761591467164, 1011766797154972559])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13628109824188894573,5078322919300450867, 7487643637446077333, 2137529107144961307])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_4() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([196606543539806642,17985880096457477326, 14966162265138344786, 1819385494261491122])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16079602958066780517,16345258063911782550, 18303475330941517515, 823310919279363436])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7533459471002151559,8169834647130524127, 606619094330696073, 2158496756092658435])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5117746100054276282,16745938114909316995, 4670349229162902475, 1854312389382843001])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13277911948903032106,10864086641838946778, 17036055887342510136, 1212439061908217475])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2894005494321221117,13082806077980613680, 17704546092404224858, 1083640560946693634])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_5() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([703578030515266563,25496294018460349, 18191093767866521093, 3306408229931931708])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11362580823935060488,10454399483519471314, 15627968621971970637, 1364643878716235263])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13073454204399567381,8040653867838793840, 7687865984297693060, 3184737563055059755])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6721441571412723835,1802256067105994630, 15237186365836057991, 918685782385244956])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13703777489160338518,10415326915293713151, 16428098593383136710, 2319380393019171231])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6534941748827582720,8360056760004138432, 14392748278828068513, 2880162854550821019])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_6() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14873051125502911514,1009876853363886420, 4474672823947956892, 1115428505259068685])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5911079613135693550,3752749518593170165, 1702720058421788200, 773472582323106912])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3365317969233036961,13899256385632020955, 14693126453331906455, 2456510395306880828])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5082564405019267636,11825294403552295597, 11947230167004052583, 2557344916016914975])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4010285659833128827,9366038021724208697, 17244127176746712683, 2354994120028153902])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1046040152624240499,1480684715219236272, 4684336044629669666, 3343979769037330885])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_7() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15098112699702984220,2123998495645443857, 12581447762523995913, 2815345161873658735])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1095543248303835792,3496951183817949915, 16481868856323731536, 241337860201373924])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9408046251630770293,14882334420031299786, 5717716342106606209, 686896818284770725])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15362283233097605671,11098434997164797739, 17383458388525969442, 68047404047513445])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11294270089242924949,718763280038911942, 18056083307454061079, 1785412206785991417])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12810652131487500521,6277236373485312474, 14186033019981525327, 1394687375082768552])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_8() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7765009060636357253,13401838373198063383, 10745983277017039672, 1166894415835160758])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5676072538714515424,6581015958579816077, 10027701725242686947, 2177029338020510606])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6034735284621204279,9535019690962812834, 13463218345857104061, 2869881645709765498])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4397002313602375938,16948995203036652973, 16546289637167983688, 1767617647095544230])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7361947652128534151,14778917046290863196, 10596147174376692470, 2310984058449825809])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7974230434340523634,409696013018667120, 7552397727242232399, 2082826338233790789])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_9() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6189008858217910749,15981568259901065929, 6243285452365789747, 1253824967350938787])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10487255409570183406,6304547224673774364, 8870835955336515481, 1029744316366848702])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5762812241648845357,17902637610707479894, 9043977758532376, 2745238020371183384])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7097482559699703523,161424245079125574, 15347927138615451900, 1064079237995689251])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15349439371439447402,15458609145310927081, 8090273200352720564, 3050841052444502077])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14011406550778045599,2784224825090364190, 5233838574089164485, 2503541737530156221])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_10() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2024175205830716529,11298551741129830328, 812994085183944913, 2974018592257116577])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11590582539209406938,2051347352089048199, 10941192854829782009, 870047992847304304])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14090515173606491781,14390430350632932198, 4998577790281400474, 234863385316542244])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13864149528822518329,17719564188760010417, 8266903160960332176, 2300576160569119546])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3677961386503261954,4444062871159456869, 14316966955055427949, 1389163690623047670])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1360449930139192542,17816559333437545525, 12447841055824122744, 394981060814630494])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_11() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15928041071838890478,3075273182791851121, 7436339725014566919, 1071504924918161196])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18073580106910977912,7734423619174067379, 11725795682358821883, 2037998566460915188])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2155659232791889495,8757482858793074453, 16943348725454012663, 2656392140705264951])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12912158878741889751,10653480401404386162, 3995801639613789632, 1718673448266899053])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([308416573176999740,501777483915383243, 17688490493280231440, 110047520664234618])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3595367639061940680,6737304213065448655, 691051244050194690, 2701389486650915289])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_12() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1140876808428999955,12418832734293113461, 17051476581336775446, 2967749011324653949])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12807970079596866176,13060982888087903162, 8683060681818609995, 3209574046643800120])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5981495732715885267,10101676608620199156, 11309004714807780781, 1670640869825258826])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5196128754950320155,14885838705051849623, 123173451381892306, 3259623516285976411])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3575769837905995914,15688651666757294450, 9077394341369938765, 2558473726662700173])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2523779218360937327,15617413458632733204, 17274461069707251335, 2976159724379518998])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_13() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8383347546047445789,4944568155102912429, 3432928317355286378, 907735993886099128])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9838063689320514173,10291428720976677730, 14586417120524520650, 2786385127210688950])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7246586583263885315,12409802738027092842, 17337723206424899556, 2420620332863605522])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15092536674585645292,10497371874814288464, 6243808119018287419, 2216998312117567794])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13772206747618402441,2532150348401102058, 754459432514738937, 951456467026354271])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9351179115738453967,7516538748888579765, 985293036292352059, 1483521329466228854])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_14() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14914165533560598666,13956726119048185260, 10999299659271223018, 1405970909956619556])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([739375528840344414,8249821931213285775, 17802380569701553571, 2252342307130469719])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4418394360382956316,7263226511067650466, 17872616104513457226, 293986572634910212])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16461522839556246942,16841045234764397809, 5196191487693545576, 1812110496667367229])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10181339830812521178,15056986997794925725, 11376528426718050729, 708569253256610337])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1466144696064783269,4312019107080634434, 14543699298973459425, 3227547483164287316])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_15() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15657738822285261443,68155775413206457, 8300795415003364282, 444270265792035930])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15783206265907704104,18229746357674227004, 7152769220919661920, 1636613715130651465])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6206692565867694754,12857285447920995208, 7849985153398735599, 1719984858963383375])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8062661333892449452,1480014193296636671, 10051358043468658316, 1368704814572432987])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2547288030905109209,7154423098523185085, 2402351836447297582, 2762755844975780820])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15672113953458100700,11640535050080497572, 6831823802014544461, 2076705844617540908])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_16() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17376968171686385021,5856658213343157117, 16046016941277195076, 2270143653282803790])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18416235843185046927,15147581033750472596, 11696540938560868952, 329097959316077673])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3905476189606537453,10293007721882021580, 13090556328511351067, 2374567828837477278])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7042160561219764429,2213543005215556150, 3865624992109602931, 1605295282027387666])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15469112229541776499,15181568572310673200, 6229895531057228586, 2993003136231493861])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14898123488853091422,3886848290220369825, 7837256007388285222, 1655851205613567734])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_17() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15046023099699898892,4114896696419805040, 996208763414065556, 1182234506039914401])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4175226852958921584,10865126619324652454, 16620945814990503487, 1852867576460981907])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17585369866175756056,1439499262584134685, 10942891596830626112, 1040416813530842675])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2806805808708611501,6361100173359414157, 3442122164700029016, 3435104725902706959])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11134770837480029391,7341186704131314864, 35185097798168410, 1978454660628911014])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9006265136227916264,6317597379868220624, 7724184146633688485, 597577324154748372])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_18() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15255632024518828682,8541715744317323958, 13398158102627447041, 565130169263801261])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17306931009475422278,8560378034454339040, 9815334764136101141, 2938942394497112558])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11927873063132582606,5591565643140140754, 9078975036975332719, 1661760286694239449])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18102757452561551850,1680781658269283815, 5360998930851725642, 3104958345473370700])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1565742430559912221,10867752689904554746, 3859815817214956292, 1354239425798439771])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3776409758376270287,14141237093925952915, 5868994824595331833, 3204413455110715295])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_19() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15085090368209596380,14362454458263421491, 17943154421655010893, 957078966421250366])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16995599389193049894,14509586367904103637, 4677600564225760953, 2501671859418590766])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5116443804637635244,18110743639686563164, 14033670162056454387, 1171632755932412795])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16219748425960438273,1357064500541257019, 2207946769962693560, 723497684009508271])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12670155384110772285,9846220989778795622, 1602150231413571397, 449904997513510380])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15224942724093188593,11450287574198372631, 16782825999229474032, 1507652605700597674])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_20() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15223113671527977362,2342299617727466054, 12474851116324980485, 3100014379107048288])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2824049481843128403,8844995764219140398, 10695270444909458562, 331118944307824648])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3587641250477769295,4088020216882040811, 3501904381792796624, 84096650943991164])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16421793351147649616,13361285441221325560, 7781942990711742932, 3459666116199515024])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17188904193828862873,2152766112226795101, 16212552342747542665, 1951284619564173038])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3903448006237362861,10685885623456165217, 8484269946533448614, 1275142031455051734])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_21() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9318475764664316926,3592933450483657813, 4282457622108167970, 1211460557319063090])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17237793885274595614,7892473786497496437, 6472523636316577374, 1349075859838035674])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17546792964882360400,14519176255571613617, 10412748147826471874, 3104111005421972360])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8578565695345865978,3952464756406399537, 12521745887869858060, 96753351539748128])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15589641271261373724,11029411258805683366, 2808607280445550225, 1210425363978004183])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14806370672499129786,11646909934829774768, 14112589315010411657, 1833827056499616531])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_22() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1527181439042904593,11163500837306557865, 5391578743488915046, 2499213182351278670])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3291607929230853092,901628070018803912, 5300070931620757570, 605761129510987602])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13889742097729655031,12575776527970613275, 1344468745259768315, 2527084202309665496])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3648123464817611092,16561037001184067534, 12661804403792919271, 3272604110879876440])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13945166727897995226,17330466911542669318, 7491648352087893971, 1480388954712272244])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11210455385670526805,6591452542609056243, 15634106585484715703, 1045815904781620634])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_23() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2227234894879431530,4010905575151312794, 760936838051432392, 3175052809469836514])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14576564797515216249,15589646365418629615, 6080623008507822691, 3337737815483554063])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15505655548831327169,10792708918904972891, 2503504878879957672, 2394160314787583417])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10593109819436207852,11777412360144808063, 12080482309404765018, 540494404208755923])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8629032744427742752,5928190717951635022, 1648144952370894740, 1038952966125308135])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8641859671332397600,3589456114571667081, 11577661623166787309, 419310496590689728])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_24() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([578449116134846814,9586594399748834082, 4383861915208305982, 3201639686785363814])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14520220902827923673,5566153351204863262, 4073345374267171479, 2751188990374701123])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3125986818094723608,2261596288480582592, 16834679055793021665, 3418253489864327050])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12895761246969355833,1935910487987639754, 45081719484199529, 631949175081581335])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7433175608280433677,6874281030596634694, 1125489927888430976, 1660933890656440433])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18338707469081262222,11946456912082239948, 3607499872710999260, 3340109266441898408])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_25() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11231489193357139589,2899043192944796514, 2600649177449062219, 41085298622995405])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12665631343779888876,7908594475825751931, 9381678416356407679, 3242702693839968608])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14400888289068000478,12829749378556016896, 9070112371010377666, 1187244616431801643])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16343571507749701774,14604737474516326093, 12939303047869097143, 1060189178296684731])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4261208805135735704,8702535457019698296, 16157729025504896235, 127268845758625022])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7849326100546689942,580205432132322201, 4417842138762413523, 1439962289241646718])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_26() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11872129290796866075,11951380641579333375, 9330111285438962004, 177313448728960613])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11010896055130764302,3830077943116342023, 12646023864002962491, 3065186174609959955])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([398692932378946391,16959333719219308294, 16803193108826789594, 3291130179543887954])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11563425622068688006,2623913257764375275, 16916695455868696677, 3394608322725119172])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7805830297007463986,1597889515826092585, 15323538536299999934, 267557155703691082])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2736225745151205553,18401284092762731168, 12191467534974229925, 1655268090508961481])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_27() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11263313086077114938,10214854740427415278, 6418010048200987058, 150356912283304968])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5869919614138297275,15976789715945737280, 6139515961092418356, 2757059893212449209])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15619787114499935992,12422589139888653344, 3139786850270727672, 2557561974539266283])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13674726832604238541,17484360641773239455, 4558405788328528983, 1017814622230232319])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3764762573963104430,4334676834291848590, 16312610150683176375, 3288752857947520466])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4762777467766658919,12785637878982654937, 6661610609914439317, 2476722788715321231])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_28() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6685511636106217277,5450338660731678454, 4691009258093479151, 2738757276888414426])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2582234111940852498,11522707432757043728, 1290329308692819001, 2481882936481679286])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13537906055817981802,15961664092029259634, 9308369065511682648, 2912125829736950842])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2319345332969302702,7086377861077739300, 14631067105130664496, 725357475937217127])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3271956660226625976,5629377588852027083, 7501664095318247249, 1965324088153325146])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12313459896439326821,10426509584854915805, 13472379326651306768, 2321535073164190607])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_29() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15539301811284282952,13980938592882876977, 16498055396064261364, 1097425325963935255])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4290360816960300160,3019487382926967816, 10356570904582277829, 1437759718929780225])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12611207181745080521,15012645216543495701, 1353482703822727997, 3387551935340331977])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11401741488134332953,10004537083365317742, 2728141916421747039, 2001904061082260821])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8724383041137331547,8484519886550612173, 3727238445428788464, 3062328586953548760])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7945661760043780633,13845243715396318316, 5066408503668745121, 3313312345129009459])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_30() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9300559618585414987,10187637048287780300, 16026436133951362651, 3259538666521590031])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([115081433818978165,4307549612641369191, 4722112827542878700, 796698116024996703])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17555212015831537346,7365637757574685928, 5637339887142762525, 2862769493256239595])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18293835190131642641,17292789635802638417, 12472319699515977939, 1120282044361977478])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7322034701958122399,15086143689036437174, 16587419990238877928, 222584894745976235])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1618872254594557433,9419356478323107901, 17085923531049601705, 2504183672891116131])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_31() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12831009428898320829,6826796074429894905, 13009425110825686879, 978631778373759954])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7794220838953106605,14192712726933787407, 9759564643149187394, 3417956628930488945])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4809406024273070660,10391646268015557259, 5709688303824866421, 2841867016509671339])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4471218429142772090,12413807207002726440, 1404396362665792355, 1489105914374371258])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17405244378498341042,7490205211402504504, 5029852470004908208, 3248461453319835322])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14817540725754898795,12277300138154331043, 11304220738004360324, 454147035458215591])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_32() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11550067988285063463,4239069616932069739, 267547343142145326, 848731715306026285])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8871333604394015413,5350813003299509583, 17170818808949654931, 3094087161860811331])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14468637713962108199,11085199147922049516, 15524297124716007999, 218626812588708317])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4088204971995579906,14835732423335694744, 16343089936520427400, 923620951259127928])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9133839688442044268,6150482178916281258, 11474718886221859558, 2585645620494314127])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([46715474624342869,17172075347145924572, 5141964832790811992, 1308716899869798579])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_33() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12101907953857955094,804270850255217544, 16021498494123380625, 3372876194569992548])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14254697355949178782,3649056278752167548, 18141671671715122219, 1896848679757650320])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13607564992244195513,10112102415904893398, 3464777952030488718, 5666352233856817])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17008410925816404488,3025902772761439371, 18178636131752825089, 1065293643367440074])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2718933036784099303,16182447629765828044, 126178687776602169, 2771538622686804747])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1512618515531514628,10459780005165477383, 2286003738546958270, 2139331183067179780])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_34() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9160649417742064841,10948492833901953309, 6790093674882337611, 1123711730375443421])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3834547086933244955,13063058915429878943, 13801611092677156168, 1266718290533501476])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5254367427732269481,5146977139622311983, 360898131560647693, 468433624433948230])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17235184452127903102,6748436236016031862, 16852424379999681833, 160029968098514362])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11215478306845311718,970258178041336956, 2420621562958120321, 604471670215399049])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([948751234125386713,216530956731643843, 5381245057587603721, 997141649891970349])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_35() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7953534332783753352,14313630231636099958, 9059042054977064594, 3450345782045885063])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6732129444676916779,6173950057435593289, 10208998668461352616, 1746743563097392614])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9780149413630146944,14286410450936261499, 7238227808382918477, 3208142555193505575])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10175469646277308276,15139298115907746396, 5670919014011340905, 2373927981440101396])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4604824893479102483,14786532165309833270, 9626430742346762057, 1865005713076863545])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3484035724721675720,11747713277245372429, 15061379510933139667, 2539567981396170022])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_36() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5116063833852600791,2413829664721887181, 13934430852741643253, 808389480121395185])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([740374015123261048,11259179314130693944, 9656628310777959662, 2385770726647698717])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3728308210645585247,16778929840491414674, 16376611935796065014, 3028224438628169100])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15268040863278363103,12966194456045166906, 17272171204369937056, 542669341054317466])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1437592663028627445,673000683127513865, 13031311178273583985, 125912731608016664])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17632969257425900530,12865415441679712422, 872220887360935278, 1892884976872251658])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_37() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2755456602104177048,158132469083934771, 16688592027911606066, 110900542082204317])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5127424667563277998,16443766868166823589, 13501088765245131741, 3048928626724254238])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7010202312460111831,436679657545091828, 5272364457133845331, 2403433919087711757])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8120509553786247155,8958622346429252603, 18225515552136326534, 2395016613717657441])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13542175247240466715,15159866924385166071, 1382942671912723302, 120170509931589891])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8231868137562833783,11714766453815983599, 8165058606065174818, 2857068324709730593])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_38() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6966102033908221466,12947831400070548925, 4850283174700431328, 489739309561298529])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4457678798389665909,17678705100979729425, 6711316111767856514, 371868678977447630])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3192336325357755526,7295027020903383045, 14355855235798742038, 3484288222187290709])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17596182633906033351,14030836130940442854, 4559642842580534159, 2619976989584061459])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8331638873159131372,13266385375089893477, 7149334849560337552, 648915628449655162])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16589776038270463123,17499135131900987656, 15175970166988992530, 2513286120975441406])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_39() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1750417144657207300,14765916003294875395, 12694390313462981605, 817497940846715459])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5010041749401916116,10131606068404255737, 1860968170564009005, 3015814904447594593])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9713149139636161576,11041296205083658948, 3580333094006591846, 3342538352087593774])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12958266191871635727,11733129801292852272, 4154495214930079271, 3263692563522337088])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10653240562461197906,741564819879868991, 13180712464205820510, 3201600767793884911])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4920588663481886378,16329868520076850738, 13972637651671967669, 2144868117784074273])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_40() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12611719975378540767,16082727358035830828, 13392582697512872398, 3278774485444707102])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13855014482826982909,10779504192157116708, 6285917858976812872, 1449582103991075139])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14202932776598698761,12875348673069514885, 4088946515024338297, 2377191937347686425])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16013150072973475429,6899119290479485872, 12522597347192296551, 1685389878151015861])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15547888574935205134,3338269424455734046, 17147028808655433463, 216265005004991423])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5487336495108463615,9769284908897020959, 9393586393066472512, 609859435096886826])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_41() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6469069486184449219,5209006114143264096, 18368135634804373178, 1515515391845588438])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16109731022265367915,13252967840910831251, 7838239659059516372, 2510179029009021030])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16714771747437637688,12719334666949756415, 8095030266951144948, 2386038909186804477])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2541294865878369517,2761251478043305762, 6055546511420936564, 3268049059449812014])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4279897082788846377,16728784809202141464, 16238028228091028030, 2970619719790617515])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14421659362690840653,1007472137087764337, 1444452226863908023, 1587368962236809553])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_42() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11320211427108126972,7973928145933084646, 8880131845862038322, 1072766723502426867])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3272334130909193463,3821649663104648296, 18372851663158092000, 1446679333947875590])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4724860141059769931,2311961596433865982, 4209074025326684959, 1220004681840620685])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16914432272745768142,3796490196977138660, 16344402701134064117, 2477442717292044976])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([649032147167613738,17446191976904625098, 9348939065808479338, 1715567241942449399])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4333410449655810169,1520486929547949309, 8810877500456747169, 1204141686730113574])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_43() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2251628061360386718,16597943346795046674, 16675129376158434734, 847045379693335785])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3917733713248981543,13697720192685540616, 13910757580898729252, 1488989016657551255])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5219305442265984379,16371098354953182443, 7540009637613607834, 2746953355326127029])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5413860454209359971,12527780751753146804, 9944557614491414119, 1147657150724542344])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14015186619611940105,7166578417100931514, 14126926957635750612, 760775902249851255])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([583695693619165758,14108604628450399737, 14389981932031848308, 300303594771841701])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_44() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7525128432213105771,7613275809558367199, 14917984546667916011, 47201003232116096])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16909181419006003082,15106539836920871237, 14516660571764972593, 2626066429487015666])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1562042360576974290,15669706555815560174, 17279858203549632100, 1874439395282871493])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4486484764872465151,16800326571470844152, 4027641834082210157, 1959040837284640996])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14571998644079463639,6392997737901797520, 7802899598167055396, 431054348366011588])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2583328984942656477,17039449995745903701, 8281333111628128422, 1196441786287267859])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_45() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2219069076238547672,17065831312995598202, 1545977477053518841, 2406397553990454831])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10801966987687445228,4716389244783281670, 8097702675466001924, 2364545576742999930])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([32724567501666414,10856478869147866873, 5759848929632126556, 833872796941750036])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13782314447538709333,11597796436851605563, 17304996645007577381, 2083525738045104495])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4047302208511501801,5315301884879046184, 6543944584213646864, 1728326958337251967])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4933523001261495003,5431877069570083092, 10815273286851991146, 1178464023994696415])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_46() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8027542791217751089,15409819256672720805, 5485085515933878445, 985352429485435282])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([599013266969562959,8189735302662428322, 14916097399582410996, 2609596713347409252])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16470905107433486047,12567406176566732030, 6972088906915651400, 158957379331098809])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5933422337405470785,18276248135291888599, 6427780681604184022, 1977859705701614693])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6680258278188696511,13009565446178638029, 8034220637928078063, 1193550928709899156])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16858929975232323281,7048751706809114116, 14419329162844326920, 660507678760628983])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_47() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7304593307160892967,568171148692076044, 15457084758917484086, 2799063040509563130])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13908761915169272301,9378125835520858340, 17827361043761851896, 1013335262217000593])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7174672501013198537,9441562095184410804, 16098437548119867353, 2551160020869567113])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8089224759845326674,10447935594062294897, 8345803794731990581, 2555879592379624793])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9995160977907629737,18439946920526240684, 16199330129884639979, 3417701694239255537])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6245705943335126646,7978583930735238696, 402887044460838417, 2819285784852152856])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_48() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8650967985165649577,10261201160683926668, 299896237387829038, 1065278328894315276])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4377864019031875809,614905911897073917, 4356350696090368122, 2177990552768436234])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10292095827930076162,18074350695342993040, 15273541425242613287, 2463294175664362057])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12004040226067164576,11180828698238295877, 7627803288434844217, 3150107166814994837])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8219482351419306013,797013333504725513, 3838063698405703643, 2200690376950712096])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([198500368808027317,424519903541480074, 826326901081780677, 1940871839773067945])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_49() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3867004862153422949,14656406366105499896, 14874358936871540821, 1730869251407314092])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11790164346317503224,10249416278226178709, 10567934763608345063, 2680021617000877166])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1240963934442374043,7840911715735611166, 16214190961312950796, 3268983596740087352])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18175916428056496888,5349553566504289891, 15508973385396330824, 1996183011196131140])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3991460885259915400,865352405290215323, 14826806539704040377, 18369765808645100])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1588472144119078703,13307094590939738044, 12563867429580768662, 3031395372828118802])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_50() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9901639564248751020,13468922798829008871, 15063943454300261726, 3107423034197502409])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4760712746226110696,17328703075444116734, 15704267607210469828, 999836410636520851])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4147787318247886546,870046555776810483, 7512520981292851181, 2169531562656440245])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1951772211207088155,10863839002701364973, 15463600760872190515, 1200851701497665728])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([419479881038790172,12029682060563011849, 8908336505807552033, 1273320352499147925])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1886095655424800325,2158138339536329022, 10818045920797858843, 2684975161178769008])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_51() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1445674089623391440,14513476766323137806, 3523554738475843262, 2781569315292901933])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17423295913557166198,16496435290478131518, 11225571173076327589, 1207909984799807022])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15185427603881017978,17060258031387871062, 226619122811737055, 292396163809273456])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([190770896663960545,12367432174244366038, 10320620112956831643, 987077250978263047])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14809221245140676799,2059596144533394624, 3536482791244349428, 3271475310347998678])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15959947944053301913,5415520623920728275, 2188159852389285731, 1464673808077357159])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_52() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8046343792035150076,14926303404203766452, 6358270153684721794, 2925220488825775511])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13629450473636744434,12007701822612359769, 7479448045435189504, 99278650264337712])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18446058239366699863,4041171095201061637, 16275339503292154106, 231618461637339883])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15410900737442176113,9911044503517126323, 4748187353539972985, 32385530837401061])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12337378394587171812,10045145253484797700, 3354249709796874811, 3073882722820279210])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15561209781351557142,1443682145804476934, 310176736937731106, 2189328421203660398])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_53() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6439261723766164999,1048860406742582089, 544484155956185788, 655768462054451379])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4484171340464876050,3485661571095878484, 10161288716950617563, 233516424983353442])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8917727502472108112,5787622687193428624, 17679166864880179465, 819953791892998429])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3060329923476031813,13726851788093064320, 1183768605439454149, 2630586564231919391])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13678884296243900156,12340423816073898674, 7818763852713622576, 1283954541846964414])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18097851810923359799,2926562898185036988, 13999800859573785231, 1030377079792608078])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_54() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2077981645701011636,5820955097984791878, 3671234845441860910, 2429880144365673811])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([400908347547812053,18438174562702003837, 2108936377451728318, 2665876678009271096])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4557040433540651370,4581124459275413115, 7544127101140285112, 2208823865612313658])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11838950012575085012,6384719434767726176, 51450923434251944, 2186960734699968405])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18129435018051483477,4979711978671127283, 17539960892240407995, 1224545171182748515])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8179268414687564123,510588373177274772, 7679199952892616989, 3281293778761246021])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_55() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8265473892133284255,15349053125371709794, 4465164319310319511, 1570901103739132822])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12049335363154888822,5119116390567087256, 8812038106240007144, 1911301909183308504])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12212193997579558737,4303759307178659250, 40966358031139690, 1930017558780958927])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10977187693411413166,17214953074262852535, 2898217150996508800, 1529733629949815919])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8563415000676559383,15266822509134095924, 3573920722763846209, 128136675083197840])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11074153514343956069,11305764762414004938, 8767476253287494619, 2707794516242251748])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_56() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16392599243409338463,1651966790311151776, 17275600814834109056, 1956689757397293497])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14343169753711807394,10778041961191581538, 3344016173878956846, 2265922894971424271])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1366080922735052523,2241627485595022347, 13186082349270174735, 1289298312940094994])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7159365057333771582,17978293804597962634, 14018798038695703590, 296794116086573737])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11066023126043827179,14060786797061805608, 3388421561476935990, 174049992596866760])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1886751431015062812,3444875023723296516, 1656450077041855680, 1121455627373805100])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_57() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17594166719163519023,3924542586583929452, 5577527157050972774, 1313677443016731129])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15847105169698284224,3608703501209440413, 6774686406383060371, 2334600097991236007])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11455851820249378835,1866247627337871854, 4811304074194598176, 3268234478610326524])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5449642234421122688,9484684970336377258, 11989070801777449448, 3129692473568484441])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18367995292201079056,4842996211444835047, 1864750310588308068, 3378033010369559879])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6991190185392338974,2120894387384959900, 3384111639118772240, 46125675607476770])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_58() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9864187871722052327,17851289785929890418, 744544961156616395, 1392402080208398867])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13638294001074905064,3451379336852867135, 14409321676187660974, 1893202060938088470])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12320456888751315737,10147107418440333983, 16712911701261196967, 178102526799022645])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2338939740287844739,11255927257596957388, 7870715421960592233, 388726270021332221])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12380885920114959312,12230873637365108260, 15462066117220621591, 411790732927354696])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18002478560433340861,7764674843260460936, 15843477938009344919, 1167025219302991094])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_59() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10038274177300451904,6188470224838173324, 15890228846675776826, 341351665721665040])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10921116401225090684,11240791833620263726, 15546059215668167409, 108531357730739792])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18274577376090422195,7471605579075004509, 2273507520301246070, 2044059933288455414])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([610168333149041861,14786949698355281727, 17395601744462790542, 2764608896271439734])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([98689349674001158,6867488731987496057, 9868136073974010044, 2415824560900660733])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10571067686588160572,8658862139835330556, 16212338354361101322, 2466918948674091048])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_60() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17322298859841239422,14602551710093018887, 1601040205264093573, 2858615701617963804])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8638599033835245049,9473006026890614189, 12427603663927709845, 2853738784762583115])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5358903601855197418,11521933377919972169, 10423732516822166271, 3313331075876581356])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17003719668296082525,10237648354402809957, 4725419000741107978, 887647791065249201])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15837410255092003678,9214053271462074546, 13631904924638840134, 2148454931302712682])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2343338517869877466,4751191999582793447, 13220493767038073895, 224626064048232768])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_61() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16208254304567926330,14821900935242021656, 9323114574626718231, 3256548186644752370])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9420793943230449471,6000301756780332082, 12889082860109228668, 350742524528645977])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15510171682382855522,1458557020456949162, 9787274640776860977, 2588602081112476500])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10248596168712541555,2861637137281799323, 10226418838670688968, 1821432979873228189])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5651218775794607854,7584115245664322490, 11761838353921556847, 1059011898424742006])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16116226122133087647,1994473248236099033, 12528494436564640920, 2068500465821749032])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_62() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7778429686336339196,8698169714979923457, 6005908535916718936, 2068150535434100632])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18235903225790823220,13883421491445981471, 8103749141318412423, 129475264728677208])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4897326650742974252,1933573767170176529, 13398015757347950597, 1301249285849189505])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12778176054771992580,7423666465122447782, 17778374465895310998, 499738461320915859])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1552418244062679524,9851253211670019312, 6322008393219437266, 2577718721302457179])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14710354835441485981,12991906282020368994, 118875214069158612, 1067070925161161299])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_63() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13768301249453257010,15843949292805420455, 10153149764899269635, 207366675836326849])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3793341162713873266,707824681465068177, 17009915741169963156, 607735161527649058])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8146072445798815213,4868320306219433228, 18416474890134888579, 1611926292238872523])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11539890178489891510,13522882435530002963, 8909705416664327832, 3251384638032127753])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2872008736039809783,1861973589192821614, 13835427573568219178, 483461972967363845])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2828765086094935498,15786319097444432030, 14470946465072737397, 2817528351934540771])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_64() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2512473857749052652,10882736938398226176, 11581675890369798961, 563285887024385670])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16281257521940250,6223206970350714564, 12171463809934695109, 180193048653739112])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14632376582626165378,1819515432877360338, 17438989287113868728, 549822435124935378])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16623803202949586117,3084999543388365758, 7066718121394421881, 461889319880245917])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18299316877843558538,4701507926792528137, 6480457203607459659, 247435578859662305])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3192926302381944206,3928810366119576278, 7561456292280096788, 3140326648238918695])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_65() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14310049893118116840,5466307996333746044, 1752376206580222107, 1749722730506676630])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1582470344854944900,12344508501334005781, 2800730537279283056, 1941647488103079913])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7873680220589352295,8679302378806680770, 6373940049552850333, 2023282579960777032])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15561565311728330692,1056388861702284265, 5306317693664828834, 2468910522551448159])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3720625826750472006,7545465479521559419, 18195952167651846018, 1877811109529315413])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15331362291223415737,16273430342432879355, 10035002333482613807, 3225407522794716678])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_66() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4985335759298641585,6313439008847483852, 11304159969539194540, 1934336312700660340])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([96271546139490934,2094043018270684037, 2490118674019965066, 3203652962092336946])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7483303426298234041,8298151461493638012, 10864150648009490293, 1752185668689004629])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6584519634585645955,11346849559285749660, 2588081768029532565, 3335412763656349857])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2282558069377451355,10489457303357466640, 14023347404052658036, 337109590688885159])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6340899982640822037,6828457521242700050, 12347712197153483551, 3037015383454642719])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_67() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8922911256223823737,8758656363816410548, 5309689272093989857, 1438299772198027897])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13005355897279050172,9464424829942711432, 10292093061671838430, 3295250916299735445])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17810740307339497148,12774691576546467211, 5155320127126655349, 3127151675654126051])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6294156264626380119,91364724648575180, 9712579871845693248, 3045322060859350367])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8150243801213740537,8891068656490762376, 9373838693523647379, 2005701092930221946])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11828574655875673593,8621613599090748602, 14603985990954990047, 2712746416027790846])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_68() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6580571806551175295,14118341263651143593, 8655953157140108767, 887252760764493841])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13009333193494997562,10189136350940921913, 16524340986026450950, 2337895636695578733])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([554602791283849967,16100251765663377100, 10431091739427625859, 2615163053116807234])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2233581346485942156,4122449169909384073, 8346659600400493386, 1067956502228151805])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6681023466273896574,9247543671285232474, 6749884543975439783, 2572242867221658005])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16892900165129469111,13345293318009891212, 4795388036930576985, 1036564656602565298])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_69() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8032622409629066489,2444892720749068261, 17366105542732113805, 1133714220223752609])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8391788813883877267,7179517711192185342, 4170425680438959642, 2741924846335966490])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([259328409317609518,625410160689016444, 2460976727230290361, 1593069272326268428])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11667986859840105587,13446619636474671790, 11072920317543582890, 1906960892196769371])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14943677003069904252,9800081198151578919, 2964892940972121265, 2004905962548161354])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5107907910479579624,6328324668983422963, 15266634576339713948, 2574515478750393244])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_70() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13737942937623694670,18260131229653792057, 11973251117894267360, 3223806620520022909])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6487034974349406492,8396259615110209112, 991327033796118938, 1867154102762685628])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3612712281436801441,3085471578915356458, 13434706423598393490, 24242710676401533])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13254416094480716295,15744647819970014461, 4995468500187085770, 751417991740609043])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4791434016930883842,10471441766425811037, 15706448817208175703, 3063104852967008449])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7115156883188573758,17045807650137803184, 5363913397289918655, 1927962398743949479])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_71() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17914206285069792197,2860965507439092907, 5976355000077728676, 772214270459130626])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6128215071866034628,18282916899309761214, 8315534576745058251, 1034870538817271661])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6060786132832099634,11135050598898523527, 16687910337997068735, 509899382268725237])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1341034480540073844,18302218830643477608, 3189662097032187086, 2453432544989212473])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5214982552257910679,16090857388377220411, 15723503505528996876, 21752958561861397])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16170916452592076337,16916134869951226311, 499080572723037059, 3076057834694940549])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_72() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4488593089637827,4256105029134743519, 1052444604106074494, 1054490616049670304])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3313974933988995404,11615449759129272165, 16274541866414324345, 1418998976438964278])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7074592618603940547,441545475921853445, 17148608621178929895, 1516692141280076905])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3338123884124467076,8099403712528760132, 2761076765830966211, 1416883715717061579])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9759829683130379807,1687174716833266171, 10851701095100941003, 3020845583738985801])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1263625008658665067,17140094019922288475, 2310524342473816831, 1288752840617157125])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_73() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8990001186161663007,2488854395876159321, 10138137362643666313, 2501243184027661072])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18433615531136467127,15818799049292160610, 4032960845822015618, 1043548168266377936])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9451652896052810486,5022653070999418869, 1300849575774313288, 1959717185631655223])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1761273794843173409,897555623629882694, 13658624702450847598, 588191865985026874])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13084665462899499560,11140264496901192476, 9558433729812815977, 2116642851492478453])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3094189602738946815,2551882070922356344, 12442854748854745313, 184898689583652828])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_74() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17085363065336137196,2983923295403189877, 10663828781034194773, 1700393170428402880])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6129988116861328192,8383387461367026841, 3696926348119727052, 173992175055984492])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16889475839678159255,6330877782154768026, 4748351306611886089, 3303245889772228682])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([442125428573901981,16697660270054791650, 8454280576721340939, 2788136955644040217])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11055158547849779490,14832147381882223717, 4780924666965999621, 2427776798480590075])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2975946219689173380,1376683279029847137, 10159265452156078299, 1307537768134300961])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_75() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13492999392056848473,3409225221748121089, 8875528568079441059, 2822886024041802092])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15943418815619886630,14220124272089015611, 1690794185244326283, 2561890306774443141])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4211418890493680271,10412021479655760876, 15106089014936003626, 2262137118010589024])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3210471309374648911,11454310338887733687, 7217981845314163473, 1667110551600172999])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12758778112869540167,685710593856742136, 16164347855864222943, 442057341553487247])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8854970239663218078,8775123848769127234, 3946335189652919039, 379947515523405491])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_76() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12731043419635983758,10817782784977234131, 743807837498126826, 2449986788418087470])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8691304308950685103,12598398158479718085, 8123035027260947259, 2795705187324000865])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([2251827982356022686,4851076356136505282, 1596796225957036378, 3221897969509652327])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([102657937007103460,1771631032207483297, 10030808951423944960, 1289847075492310447])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15202837835844737580,7594999580604290137, 5274106387427308266, 1280416036119175045])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5129688266723762816,8664554382089339711, 954611873558983922, 2012040923036770265])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_77() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9994530615554439765,3407217428591799778, 13068733661325357315, 2041462547972733256])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10307368795699820840,329889077234513378, 6351010326181219895, 834699583228135399])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4150573024268527170,3947564887232855981, 4740622443617200702, 1323156502984230654])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15531277559705000899,14615009384157705096, 14736033853090294351, 872354188864157439])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12115137459399439569,10894610032757578695, 3817667239236480611, 659149945585729809])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1445061471446911457,4335860331905653289, 3950641420627671188, 3438645524104142458])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_78() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1839570843343726836,3731424696932177018, 15197846145592355408, 1158183081834675472])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3116529262232091341,14803887511507603872, 15294218269902372493, 2729177066042744272])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4691834745047784444,6290710230747365109, 8060827630314259371, 851272282068838766])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6540961978479069267,12620905532604192189, 2259804200453272966, 1165975336850311858])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3690834163650730271,2183408924574326954, 10111614024162341117, 1554210036148629477])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8289229043509631629,2064235223198990595, 6762562399217845173, 1755571685257714778])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_79() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1129606137083694000,9693594255691914122, 942184232470642500, 76754758712589041])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11787188711857939778,2291484922533402482, 8045588759716898020, 2666056272632326089])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13512721703601274707,2559328823920164168, 6551027390042314934, 3144302214298181468])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6079799944678829135,17574099226699962876, 3422347169802655098, 1879862275945803574])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16320328042042081202,16485432521573086932, 4582127847920718464, 3238131773946700186])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14524476379657058700,2104537825481602716, 207619468682790327, 2782853175465112052])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_80() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4848957945632739244,14518053439709387994, 5756081468879895478, 260862080211444277])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9921707591848124421,13545670523771785303, 14173002405173204945, 3454156026882900006])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12481624538503695733,18374782245059226246, 4636568710772954398, 944278067458368013])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11605228600784000910,7582524798767844179, 13552443886180595709, 1369532973947736416])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4267299552200762597,8751182266582777814, 4528214310785546759, 2945317819632247909])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5844873528134729976,17562793343608500664, 699835019980445740, 323666743035764546])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_81() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3304745764224466480,16142890819667728169, 8080742399795242786, 2013262536769460738])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10513434226171906409,6042945395677414781, 4701866271210248102, 1276312949421431287])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6799048877794150714,6489775924405872044, 4752590929046738127, 1823172490524509634])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16431608528714632239,10415839119617529591, 8460809962187628151, 790624559786111165])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15283293515394705856,13722620741574713198, 16042296800640743056, 1056608553217542157])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8401759862295761665,1045860981994246934, 17006614762575043429, 491492109879739040])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_82() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([468894081076616333,12927210800358457025, 4537092072863427935, 952700008854819772])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([17090282266488336236,763659825137433520, 16540195196327260520, 2239526232894933831])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12179546749471258750,635290039297490620, 1561291748574457568, 1049273342052835376])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4819454998367166883,15490199442768539670, 7904812857373876208, 135786140805826428])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([18152968326689103310,10177734322442556854, 3846192175324229328, 2656852663452338727])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3472929636895802959,15472768604273568082, 15098528643126491889, 535073910730780468])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_83() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12107509134704261574,6633819074915833306, 6071085156483515287, 1280440896819992107])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14683465715822475732,2977294813468700347, 16710273132247650856, 2865864818447600939])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8385215094017294615,4159766917617695228, 7401991117188359682, 3160023965077855111])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5584322178444831724,4210141106773558333, 9849158578693809435, 389773879635807247])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13059817227867972419,10206699072978985656, 4243224417125504595, 2095139565211083208])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16601340182693691765,13259260063961876271, 17263254687287459315, 2018894313217144369])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_84() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13856043817959895092,12254608518937170101, 13302648914648169924, 1780037056174772410])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11333129354572946704,2810734708730312138, 17758309205001498036, 361501848540871191])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10595413924876808110,657404680217828593, 5591957290899800063, 2859049692085224819])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1315848309892264979,7488658884080066229, 9607371636095021925, 816849158945046670])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14844372847551617288,3670747218122949646, 5044435316989032056, 3238817682698821560])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12568228469880725570,4307732791850164629, 15864215481642786007, 1306539561515084668])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_85() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([1273517890442355305,13194947519246955695, 2237462547797209973, 1690690315208843036])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9035439030688396419,4720748415469687382, 3081453488990843216, 2440745216511232626])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5669334031474563339,18299450985159507632, 14247870854957188556, 898259339158296345])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8861988074827567374,2157913986799838160, 5930834169729665633, 2479573717677737247])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11194830282499531661,15790583564496963034, 8512740866888674089, 468827250938827587])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([16793158479175563377,13992215751277618505, 17428176139655228488, 118713085709111200])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_86() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10679803011067703428,10593761633057667502, 4391821794321597344, 2938683106540358242])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([215356818242128692,9671879540788342720, 5053030485174369869, 2247500145589797429])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4004301083392092688,4364723893981464035, 16112216998220807300, 2455819978069286735])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6726240373041376886,18025680923636443261, 15587055391740618224, 1892997414505807272])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8757123833278971516,13944650926374912848, 6515091157474530459, 414837118493961111])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([6184673448221396191,3869443642859899199, 12295380941180649872, 2702227909921114720])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_87() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([15263258468871800828,749379425742922011, 15864883827799632885, 1997739201144032190])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12567658207603502477,1836730742042396038, 6759728617080602216, 420241556039658577])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3037776819980253614,3964052020337179088, 10801610276232288091, 3031896956241654605])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8480104532681213425,13227423875840765909, 17508151446982868316, 1337594997761420348])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8734311714183300487,14653092031957514568, 10864574748231343876, 922067468620025980])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([7624897394301847599,14196935534022316622, 10715890202663828208, 314573899055890642])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_88() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5195930244918102199,5303373722594869815, 4598574952250964448, 587535923307507459])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([10281941356938789589,17264179986976798016, 2495327132738377768, 2822867078449382473])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([9156412354495559242,6473304141841425097, 3894574450883273992, 1041158042949639581])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13529849375683860416,6394110684950684881, 17641586219368964467, 1070008667872956710])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([13426668510061172940,14820485702914429128, 11690980893900564436, 2827242719088429161])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8179779897068071171,17235144014779109291, 7689419687298213974, 1776360945914461637])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_89() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4014058372650537153,14192927669923690004, 6265023380233947493, 2364764780907433958])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14428545205121103236,5638984200726281258, 3296255194557494893, 904553563711437935])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5301477391265961812,5505677977326641084, 16724270212271456544, 2250345117517408606])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([5519988502296978404,1268113604296698963, 7710441188319123469, 1421065756653321236])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14835068654385416010,10997216849596544895, 15801418972803987005, 236865724659060878])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([8993880476618214013,15788026752317685857, 15258174033443871581, 2964200635327504084])) 
 		)
 	)
}

pub fn get_delta_g2_neg_pc_90() -> (QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>, QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>) { 
 	(QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([531450005040739183,11381687628615024602, 4147293147584778871, 1915523518969145936])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([4738548415080720630,6664909504222730622, 7458139914191132783, 3117159955370418896])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([3733904784425318460,7898651805639499248, 11692231721198865192, 980376346097203176])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([11927937031456897314,16173098780166862066, 4234484756590708229, 1791348118404483031])) 
		),
		QuadExtField::<ark_ff::Fp2ParamsWrapper::<ark_bn254::Fq2Parameters>>::new(
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([12579030105839912151,16096696663585909244, 14566235941154857016, 3054265487862999523])), 
			ark_ff::Fp256::<ark_bn254::FqParameters>::new(BigInteger256::new([14408153541220030970,10701384406538183676, 1572038441739200563, 148138379414820208])) 
 		)
 	)
}
pub const ALPHA_G1_BETA_G2: [u8;384] = [13, 20, 220, 48, 182, 120, 53, 125, 152, 139, 62, 176, 232, 173, 161, 27, 199, 178, 181, 210, 207, 12, 31, 226, 117, 34, 203, 42, 129, 155, 124, 4, 74, 96, 27, 217, 48, 42, 148, 168, 6, 119, 169, 247, 46, 190, 170, 218, 19, 30, 155, 251, 163, 6, 33, 200, 240, 56, 181, 71, 190, 185, 150, 46, 24, 32, 137, 116, 44, 29, 56, 132, 54, 119, 19, 144, 198, 175, 153, 55, 114, 156, 57, 230, 65, 71, 70, 238, 86, 54, 196, 116, 29, 31, 34, 13, 244, 92, 128, 167, 205, 237, 90, 214, 83, 188, 79, 139, 32, 28, 148, 5, 73, 24, 222, 225, 96, 225, 220, 144, 206, 160, 39, 212, 236, 105, 224, 26, 109, 240, 248, 215, 57, 215, 145, 26, 166, 59, 107, 105, 35, 241, 12, 220, 231, 99, 222, 16, 70, 254, 15, 145, 213, 144, 245, 245, 16, 57, 118, 17, 197, 122, 198, 218, 172, 47, 146, 34, 216, 204, 49, 48, 229, 127, 153, 220, 210, 237, 236, 179, 225, 209, 27, 134, 12, 13, 157, 100, 165, 221, 163, 15, 66, 184, 168, 229, 19, 201, 213, 152, 52, 134, 51, 44, 62, 205, 18, 54, 25, 43, 152, 134, 102, 193, 88, 24, 131, 133, 89, 188, 39, 182, 165, 15, 73, 254, 232, 143, 212, 58, 200, 141, 195, 231, 84, 25, 191, 212, 81, 55, 78, 37, 184, 196, 132, 91, 75, 252, 189, 70, 10, 212, 139, 181, 80, 22, 228, 225, 237, 242, 147, 105, 106, 67, 183, 108, 138, 95, 239, 254, 108, 253, 219, 89, 205, 123, 192, 36, 108, 23, 132, 6, 30, 211, 239, 242, 40, 10, 116, 229, 111, 202, 188, 91, 147, 216, 77, 114, 225, 10, 10, 215, 128, 121, 176, 45, 6, 204, 140, 58, 228, 53, 147, 108, 226, 232, 87, 34, 216, 43, 148, 128, 164, 111, 3, 153, 136, 168, 12, 244, 202, 102, 156, 2, 97, 0, 248, 206, 63, 188, 82, 152, 24, 13, 236, 8, 210, 5, 93, 122, 98, 26, 211, 204, 79, 221, 153, 36, 42, 134, 215, 200, 5, 40, 211, 180, 56, 196, 102, 146, 136, 197, 107, 119, 171, 184, 54, 117, 40, 163, 31, 1, 197, 17] ; 