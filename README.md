# Cw721 Rewards

This is a modified version of cw721 NFT contract. It implements
archway-bindings to withdraw reward per token basis. The
calculation for each token reward distribution is as follows,

```
token_reward(token_id) = total_rewards / max_supply - claimed_rewards(token_id)
```

## Implementation

This contract implements cw721 with added functionality, to deploy please provide the `token_uri`
and `max_supply` to the instantiate command.

```
'{"name":"Test Collection","symbol":"NFTEST","token_uri":"ipfs://QmejYa4kkcnCjDiZwy2YnNCY2CBBYnnxDV3V2F1Eh77iya/1.json","max_supply":777}'
```

After instantiating, set the contract metadata for Archway reward distribution (note: you can assign
any other contract to the nft contract address for more rewards)

```
archwayd tx rewards set-contract-metadata $CONTRACT_ADDRESS --gas auto --gas-prices 0.05uconst --gas-adjustment 1.4 --from $DEPLOYER --chain-id "constantine-2" --node "https://rpc.constantine-2.archway.tech:443" --broadcast-mode sync --output json -y --owner-address $CONTRACT_ADDRESS --rewards-address $CONTRACT_ADDRESS
```

Anyone can mint (freemint) the nft using the `mint` message,

```
archwayd tx wasm execute $CONTRACT_ADDRESS '{"mint":{"extension":{}}}' --from prime --chain-id "constantine-2" --node "https://rpc.constantine-2.archway.tech:443" --output json -y --gas auto --gas-prices 0.05uconst --gas-adjustment 1.4
```

After couple of txs, any account can trigger the reward withdrawal with `withdraw_rewards`

```
archwayd tx wasm execute $CONTRACT_ADDRESS '{"withdraw_rewards":{}}' --from prime --chain-id "constantine-2" --node "https://rpc.constantine-2.archway.tech:443" --output json -y --gas auto --gas-prices 0.05uconst --gas-adjustment 1.4
```

This distribute the reward to all tokens equally, as you can see with `total_arch_reward`

```
archway query contract-state smart --args '{"total_arch_reward":{"token_id":"1"}}'
```

Token owner can withdraw the reward available with `withdraw_token_rewards`,

```
archwayd tx wasm execute $CONTRACT_ADDRESS '{"withdraw_token_rewards":{"token_id":"1"}}' --from prime --chain-id "constantine-2" --node "https://rpc.constantine-2.archway.tech:443" --output json -y --gas auto --gas-prices 0.05uconst --gas-adjustment 1.4
```

## Running this contract

You will need Rust 1.65+ with `wasm32-unknown-unknown` target installed.

You can run unit tests on this via:

`cargo test`

Once you are happy with the content, you can compile it to wasm via:

```
RUSTFLAGS='-C link-arg=-s' cargo wasm
cp ../../target/wasm32-unknown-unknown/release/cw721_base.wasm .
ls -l cw721_base.wasm
sha256sum cw721_base.wasm
```

Or for a production-ready (optimized) build, run a build command in the
repository root: https://github.com/CosmWasm/cw-plus#compiling.

## Importing this contract

You can also import much of the logic of this contract to build another
CW721-compliant contract, such as tradable names, crypto kitties,
or tokenized real estate.

Basically, you just need to write your handle function and import
`cw721_base::contract::handle_transfer`, etc and dispatch to them.
This allows you to use custom `ExecuteMsg` and `QueryMsg` with your additional
calls, but then use the underlying implementation for the standard cw721
messages you want to support. The same with `QueryMsg`. You will most
likely want to write a custom, domain-specific `instantiate`.

**TODO: add example when written**

For now, you can look at [`cw721-staking`](../cw721-staking/README.md)
for an example of how to "inherit" cw721 functionality and combine it with custom logic.
The process is similar for cw721.
