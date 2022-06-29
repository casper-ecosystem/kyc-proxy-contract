# KYC Proxy Contract
This is a proxy contract to check if an account is kyc proved through registered providers(KYC Contracts).

This contract is compatible with KYC Contracts that have the following entrypoint:
```
EntryPoint::new(
    "is_kyc_proved",
    vec![
        Parameter::new("account", Key::cl_type()),
        Parameter::new("index", CLType::Option(Box::new(U256::cl_type()))),
    ],
    CLType::Bool,
    EntryPointAccess::Public,
    EntryPointType::Contract,
)
```

This proxy contract accepts a list of `contract_package_hash` on install deploy or singular package hashes on later deploys when calling the `"add_provider"` entrypoint.
These providers can be banned or unbanned. Banned providers will not be asked for their opinion.

## Endpoints
### *init(initial_providers: Vec<ContractPackageHash>)*
Initialize proxy contracts with a given list of `contract_package_hash`

### *is_kyc_proved(account: Key, index: Option<U256>) -> bool*
Check if a given account is kyc proved

### *add_kyc_provider(provider: Key)*
Register a new kyc provider contract inside proxy contract

### *ban_provider(provider: Key)*
Set given kyc provider contract as validated inside proxy contract

### *unban_provider(provider: Key)*
Set given kyc provider contract as invalidated inside proxy contract


### Casper SDK Version
casper-contract = "1.4.4"
casper-engine-test-support = "2.2.0"
casper-execution-engine = "2.0.0"
casper-types = "1.5.0"