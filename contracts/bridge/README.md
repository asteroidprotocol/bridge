# Bridge

The bridge contract is responsible for verifying transfers coming from the Cosmos Hub and minting the associated TokenFactory token if the transfer is valid. Conversely, it releases CFT-20 tokens and burns the TokenFactory ones when bridging back to the Cosmos Hub

The process works as follows:

1. To enable the bridge for a specific CFT-20 you need to do a transaction on Neutron first. This is because Neutron charges 1 NTRN to create a new TokenFactory token. To enable a token to be bridged you need to execute a transaction with verified signatures to the contract on Neutron

a. To get the verified signature, you would call the Verification API. It would return a signed message with the token's information
b. Execute an Activate transaction against the bridge contract and pay the 1 NTRN to enable the bridge

2. Once activated, you can bridge the token from the Hub

a. Send XX amount of a token to the bridge virtual address "bridge"
b. Call the verification API to get a signed message with the amount you transferred to the bridge
c. Execute the bridge message returned by the API
d. Check that the tokens were received on Neutron

3. To bridge back you do the opposite, but without calling the Verification API

a. Send the tokens you want to bridge back together with a 'Bridge' message
b. Check that the funds were received again on the Cosmos Hub


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


## Messages

```rust
#[cw_serde]
pub enum ExecuteMsg {


    /// Receive CFT-20 token message from the Hub
    Receive {
        /// The ticker of the CFT-20 token
        ticker: String,
        /// The name of the CFT-20 token
        name: String,
        /// The amount of decimals this CFT-20 uses
        decimals: Uint8,
        /// The destination address to transfer the CFT-20-equivalent to
        destination_addr: String,
        
        // TODO: Signature and checking data
    }
    /// Bridge funds back to the Cosmos Hub
    Bridge {
        /// The destination address on the Hub to send the tokens to
        destination_addr: String
    },
    /// Update parameters in the Bridge contract. Only the owner is allowed to
    /// update the config
    UpdateConfig {
        /// The new Hub address
        hub_addr: Option<String>,
    },
}
```

## Message details

**Receive**

Receive CFT-20 tokens via IBC-Hook

```json
{
    "receive": {
        "ticker": "ROIDS",
        "name": "Asteroids",
        "decimals": 6,
        "destination_addr": "cosmos1xxxxx"   
    }
}
```


**Bridge**

Send TokenFactory tokens

```json
{
    "bridge": {
        "destination_addr": "cosmos1xxxxx"
    }
}
```


**Update Config**

Update config allows the owner to set a new address for the Hub. Updating the Hub address will remove the known Hub channel and a new one will need to be established.

```json
{
    "update_config": {
        "hub_channel": "channel-1"
    }
}
```
