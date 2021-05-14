# SPL Token Lending program command line interface

A basic CLI for initializing lending markets and reserves for SPL Token Lending.  See https://spl.solana.com/token-lending for more details

## Commands

```shell
    spl-token-lending create-market [FLAGS] [OPTIONS] \
        --owner <PUBKEY> \
        --quote <PUBKEY>
```

```shell
    spl-token-lending add-reserve [FLAGS] [OPTIONS] \
        --market <PUBKEY> \
        --source <PUBKEY> \
        --amount <INTEGER> \
        --oracle <PUBKEY> \
        --optimal-utilization-rate <PERCENT> \
        --loan-to-value-ratio <PERCENT> \
        --liquidation-bonus <PERCENT> \
        --liquidation-threshold <PERCENT> \
        --min-borrow-rate <PERCENT> \
        --optimal-borrow-rate <PERCENT> \
        --max-borrow-rate <PERCENT> \
        --borrow-fee-wad <WAD> \
        --host-fee-percentage <PERCENT>
```
