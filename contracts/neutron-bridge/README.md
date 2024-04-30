# Neutron Bridge

    NOTE: This contract is specific to Neutron

The bridge contract is responsible for verifying transfers coming from the Cosmos Hub and minting the associated TokenFactory token if the transfer is valid. Conversely, it releases CFT-20 tokens and burns the TokenFactory ones when bridging back to the Cosmos Hub

The process works as follows:

1. To enable the bridge for a specific CFT-20 you need to do a transaction on Neutron first. This is because Neutron charges 1 NTRN to create a new TokenFactory token. To enable a token to be bridged you need to execute a transaction with verified signatures to the contract on Neutron

- To get the verified signature, you would query the API provided by signers. It would return a signature together with with the token's information
- Execute a LinkToken transaction against the bridge contract and pay the 1 NTRN to enable the bridge using a majority of the loaded signers

2. Once activated, you can bridge the token from the Hub

- On the Hub you need to create an inscription with a URN similar to

```text
urn:bridge:cosmoshub-4@v1;send$tic=TESTTOKEN,amt=10,rch=neutron-1,rco=neutron1m0z0kk0qqug74n9u9ul23e28x5fszr628h20xwt6jywjpp64xn4qatgvm0,dst=neutron1vrmfyhxjlpg32e68f5tg7qn9uftyn68u70trzs
```

- This transfer will be signed by the indexer if the transfer is successful
- Querying the API provided by signers will provide you with the signature for the bridging operation
- Execute the bridge message with majority signature either via an IBC call (using IBC hooks) or directly on Neutron
- Check that the tokens were received on Neutron
- If the IBC transaction fails, the user can retry it

3. To bridge back you do the opposite, but without calling the Verification API

- Send the tokens to the bridge contract in the Send transaction
- Check that the funds were received again on the Cosmos Hub


## Generating public/private keys

The private key needs to be added to the indexer operated by a trusted entity with the public key being added to the contract.

### Generating the private key

Generate the ed25519 private key for your indexer

```bash
openssl genpkey -algorithm ed25519 -outform PEM -out trusted-party-ed25519priv.pem
```

Extract the public key in the correct format

```bash
openssl pkey -in trusted-party-ed25519priv.pem -pubout -outform DER -out trusted-party-ed25519pub.der
tail -c 32 trusted-party-ed25519pub.der > trusted-party-ed25519pub.raw
```

Finally get the compatible public key for easy verification in the contract

```bash
base64 trusted-party-ed25519pub.raw > trusted-party-ed25519pub-contract.txt
```

