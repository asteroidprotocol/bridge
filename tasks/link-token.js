import { contractTask, logTransaction } from "@asteroid-protocol/lift";
import { readFile } from "fs/promises";
import crypto from "crypto";

contractTask(async (context, contract) => {
  const token = {
    decimals: 6,
    image_url: "",
    name: "Asteroids",
    ticker: "ROIDS",
  };
  const sourceChainId = "gaialocal-1";
  const destinationChainId = "test-1";

  const message = `${sourceChainId}${token.ticker}${token.decimals}${destinationChainId}${contract.address}`;

  const key1 = await readFile("keys/trusted-party-1-ed25519priv.pem", "utf8");
  const signature1 = crypto.sign(null, Buffer.from(message), key1);

  const key2 = await readFile("keys/trusted-party-2-ed25519priv.pem", "utf8");
  const signature2 = crypto.sign(null, Buffer.from(message), key2);

  const signatures = [
    signature1.toString("base64"),
    signature2.toString("base64"),
  ];

  const res = await contract.execute({
    link_token: {
      signatures,
      source_chain_id: sourceChainId,
      token,
    },
  });

  logTransaction(res);
});
