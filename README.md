# Asteroid Bridge

## Overview of CFT-20 Bridging

The bridge contract serves as a crucial component for verifying and facilitating token transfers between the Cosmos Hub and Neutron. It ensures that TokenFactory tokens are minted upon receiving valid transfers from the Cosmos Hub. Conversely, when tokens are bridged back to the Cosmos Hub, the bridge contract releases CFT-20 tokens and burns the corresponding TokenFactory tokens.

### How to Use the Bridge Contract

**1. Enabling the bridging of a CFT-20 Token**

To enable the bridging of a token, the token information needs to be verified, added to the contract and a TokenFactory token needs to be created for the CFT-20. In most cases, creating a TokenFactory token has a cost associated with it

To enable a token for bridging:

a. Query the API provided by the signers to obtain signatures that verifies the token information, along with the actual token information

b. Execute the `LinkToken` transaction on the contract with the signatures as well as the token information



**2. Bridging tokens from the Cosmos Hub**

Once the link has been created, you may now bridge tokens. To bridge tokens you must create a bridge inscription for tokens that you hold.

The process is:

a. Create a bridge inscription using a similar URN as shown below

```text
urn:bridge:cosmoshub-4@v1;send$tic=TESTTOKEN,amt=10,rch=neutron-1,rco=neutron1m0z0kk0qqug74n9u9ul23e28x5fszr628h20xwt6jywjpp64xn4qatgvm0,dst=neutron1vrmfyhxjlpg32e68f5tg7qn9uftyn68u70trzs
```

b. Indexers will process this transaction, and if it is valid, create a signature for the transaction

c. Query the API provided by signers to obtain signatures that verify the transfer along with transaction information

d. Execute a `Receive` transaction on the contract with the information and signatures. This can be done via IBC-Hooks or directly on the destination chain

e. If the transaction and signatures are valid, the contract will mint the amount of the TokenFactory token and send it to the destination address

f. If IBC-Hooks are used, the user may safely retry the `Receive` transaction should there be an IBC failure



**3. Bridging back to the Cosmos Hub**

To bridge back is simpler as the chain already verifies all the information that is required, the process is:

a. Execute a `Send` transaction on the contract while also sending the bridged token as part of the `funds` section

b. The bridge contract is only connected to the Cosmos Hub and will initiate an IBC transaction with a specific memo. This also burns the bridged tokens

c. Once the IBC transaction arrives on the Hub, the indexers will pick it up, and if valid, release the CFT-20 tokens to the destination address



## Signers / Indexers

Indexers read all the memos on the Cosmos Hub and process metaprotocol transactions for inscriptions and CFT-20 tokens. Anyone can run and indexer and we encourage people to do so. As indexers are by nature completely centralised the bridge requires multiple indexers to agree on the state before tokens can be bridged.

We aim to have at least 3 independent and trusted indexers that will act as signers for the bridge. They will generate and control their own private keys with which they sign the transactions. The bridge contract will require a majority of signers to agree and sign transactions to allow the bridging to be done to another chain.


### Generating public/private keys

The private key needs to be added to the indexer operated by a trusted entity with the public key being added to the contract.

**Generating the keys**

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




