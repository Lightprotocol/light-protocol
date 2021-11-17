#solana-test-validator --reset;

#solana airdrop 300 ALA2cnz41Wa2v2EYUdkYHsg7VnKsbH1j7secM5aiP8k;
# cd ../program/;
# sh deploy_program.sh;
# cd ../program_prep_inputs/;
# sh deploy_program.sh;
# cd ../Client-Js/webassembly/;
# sh compile_wasm.sh;
# cd ..;
# npm run-script run init_merkle_tree;

#purge notes of a previous test
#rm notes.txt;
for i in `seq 0 100`
  do
  cd ../Client-Js/;
  for i in `seq 0 10`
  do
      npm run-script run deposit SOL 1 a >> ../test_scripts/devnet_2_a_notes.txt;

  done
  cd ../test_scripts/ && python3 withdrawal_test.py;
  cd ../Client-Js/;
done
