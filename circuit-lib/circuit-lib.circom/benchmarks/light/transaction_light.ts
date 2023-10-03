import { BenchmarkLightCircuit, circuitParameterVariants } from "./utils";

async function bench() {
  const maspParameterVariants: circuitParameterVariants = {
    merkleTreeHeightVariants: [18, 20, 22, 24, 26],
    nInputUtxosVariants: [2],
    nOutputUtxosVariants: [2],
    outputPath: "./artifacts/bench_circuits/light",
  };
  const benchMasp = new BenchmarkLightCircuit(maspParameterVariants);
  await benchMasp.benchmark(4);

  const appParameterVariants: circuitParameterVariants = {
    merkleTreeHeightVariants: [18, 20, 22, 24, 26],
    nInputUtxosVariants: [4],
    nOutputUtxosVariants: [4],
    outputPath: "./artifacts/bench_circuits/light",
  };
  const benchApp = new BenchmarkLightCircuit(appParameterVariants, true);
  await benchApp.benchmark(4);
}

bench();
