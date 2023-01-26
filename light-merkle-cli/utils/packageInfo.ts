import path from "path";
import findup from "find-up";
import fsExtra from "fs-extra";

export interface PackageJson {
  name: string;
  version: string;
  engines: {
    node: string;
  };
}

export const getPackageRoot = () => {
  const packageJsonPath = getPackageJsonPath();
  return path.dirname(packageJsonPath ? packageJsonPath : "");
};

export const getPackageJsonPath = () => {
  return findClosestPackageJson(__dirname);
};

export const getPackageJson = async (): Promise<PackageJson> => {
  const root = getPackageRoot();
  return fsExtra.readJSON(path.join(root, "package.json"));
};

export const findClosestPackageJson = (file: string) => {
  const res = findup.sync("package.json", { cwd: path.dirname(file) });
  return res;
};

// TODO: fix the wrong version
export function getLightVersion() {
  const packageJsonPath = getPackageJsonPath();

  if (packageJsonPath === null) {
    return null;
  }

  try {
    const packageJson = fsExtra.readJsonSync(
      packageJsonPath ? packageJsonPath : ""
    );
    return packageJson.version;
  } catch {
    return null;
  }
}

export const getInputJson = async (input: string) => {
  const inputString = await fsExtra.readFile(input, "utf8");
  try {
    return JSON.parse(inputString.toString().replace(/^\uFEFF/, ""));
  } catch (error: any) {
    throw new Error(error);
  }
};
