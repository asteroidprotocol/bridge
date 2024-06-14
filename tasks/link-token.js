import { contractTask, logTransaction } from "@asteroid-protocol/lift";
import { signMessage } from "./src/utils.js";
import {
  SOURCE_CHAIN_ID,
  DESTINATION_CHAIN_ID,
  TICKER,
  ONLY_ONE_SIGNER,
} from "./src/constants.js";

contractTask(async (context, contract) => {
  const token = {
    decimals: 6,
    image_url: "",
    name: "Asteroids",
    ticker: TICKER,
  };

  const message = `${SOURCE_CHAIN_ID}${token.ticker}${token.decimals}${DESTINATION_CHAIN_ID}${contract.address}`;

  const signatures = await signMessage(message, ONLY_ONE_SIGNER);

  const res = await contract.execute({
    link_token: {
      signatures,
      source_chain_id: SOURCE_CHAIN_ID,
      token,
    },
  });

  logTransaction(res);
});
