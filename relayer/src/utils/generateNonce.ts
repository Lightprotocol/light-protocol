import nacl from "tweetnacl";
export const generateNonce = (): string =>
  String(nacl.randomBytes(nacl.box.nonceLength));
