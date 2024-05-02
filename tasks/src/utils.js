import { readFile } from "fs/promises";
import crypto from "crypto";

export async function signMessage(message) {
  const key1 = await readFile("keys/trusted-party-1-ed25519priv.pem", "utf8");
  const signature1 = crypto.sign(null, Buffer.from(message), key1);

  const key2 = await readFile("keys/trusted-party-2-ed25519priv.pem", "utf8");
  const signature2 = crypto.sign(null, Buffer.from(message), key2);

  return [signature1.toString("base64"), signature2.toString("base64")];
}
