import { contractTask, logTransaction } from "@asteroid-protocol/lift";
import { readFile } from "fs/promises";
import { ONLY_ONE_SIGNER } from "./src/constants.js";

contractTask(async (context, contract) => {
  const pubkey1 = await readFile(
    "keys/trusted-party-1-ed25519pub-contract.txt",
    "utf8"
  );
  const pubkey2 = await readFile(
    "keys/trusted-party-2-ed25519pub-contract.txt",
    "utf8"
  );

  let res = await contract.execute({
    add_signer: {
      name: "trusted-party-1",
      public_key_base64: pubkey1.trim(),
    },
  });

  logTransaction(res);

  if (!ONLY_ONE_SIGNER) {
    res = await contract.execute({
      add_signer: {
        name: "trusted-party-2",
        public_key_base64: pubkey2.trim(),
      },
    });

    logTransaction(res);
  }

  const signers = await contract.query({ signers: {} });
  console.log("signers:", signers);
});
