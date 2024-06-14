import {
  contractTask,
  logTransaction,
  contractCommand,
} from "@asteroid-protocol/lift";
import { signMessage } from "./src/utils.js";
import {
  SOURCE_CHAIN_ID,
  DESTINATION_CHAIN_ID,
  TICKER,
  ONLY_ONE_SIGNER,
} from "./src/constants.js";

const command = contractCommand();
command.requiredOption(
  "-h, --transaction-hash <transactionHash>",
  "Transaction hash"
);
command.requiredOption("-m, --amount <amount>", "Amount");
command.requiredOption(
  "-d, --destination-address <destinationAddress>",
  "Destination address"
);

contractTask(command, async (context, contract) => {
  const options = command.opts();

  const transactionHash = options.transactionHash;
  const amount = options.amount;
  const destinationAddress = options.destinationAddress;

  const message = `${SOURCE_CHAIN_ID}${transactionHash}${TICKER}${amount}${DESTINATION_CHAIN_ID}${contract.address}${destinationAddress}`;

  const signatures = await signMessage(message, ONLY_ONE_SIGNER);

  const res = await contract.execute({
    receive: {
      signatures,
      source_chain_id: SOURCE_CHAIN_ID,
      transaction_hash: transactionHash,
      ticker: TICKER,
      amount: `${amount}`,
      destination_addr: destinationAddress,
    },
  });

  logTransaction(res);
});
