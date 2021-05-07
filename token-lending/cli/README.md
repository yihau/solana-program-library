# SPL Token Lending program command line interface

A basic CLI for initializing lending markets and reserves for SPL Token Lending.  See https://spl.solana.com/token-lending for more details

## Commands

```shell
spl-token-lending create-market \
  --owner <OWNER_ADDRESS> \
  --quote <MINT_ADDRESS>
```

```shell
spl-token-lending add-reserve \
  --market <MARKET_ADDRESS> \
  --source <LIQUIDITY_ADDRESS> \
  --amount <LIQUIDITY_AMOUNT> \
  --config <RESERVE_CONFIG> \
  --oracle <ORACLE_ADDRESS?>
```
