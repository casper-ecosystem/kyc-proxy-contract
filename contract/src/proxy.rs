#![no_main]
#![no_std]

extern crate alloc;

use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
    {format, vec},
};
use casper_contract::{
    contract_api::{
        runtime::{self, call_versioned_contract, revert},
        storage::{self, dictionary_get, dictionary_put, new_dictionary},
    },
    unwrap_or_revert::UnwrapOrRevert,
};
use casper_types::{
    contracts::NamedKeys, runtime_args, ApiError, CLType, CLTyped, CLValue, ContractPackageHash,
    EntryPoint, EntryPointAccess, EntryPointType, EntryPoints, Key, Parameter, RuntimeArgs, URef,
    U256,
};

/// EntryPoint that initializes the contract's storage scheme. This entry_point can only be called once,
/// as on the second call the creation of the dictionary will fail because it already exists.
#[no_mangle]
pub extern "C" fn init() {
    ProviderDict::init(runtime::get_named_arg("initial_providers"))
}

/// EntryPoint that checks with each stored KYC provider contract for an account's validity.
/// This EntryPoint can only be called from inside a contract or a session code since it returns a value.
#[no_mangle]
pub extern "C" fn is_kyc_proved() {
    let account = runtime::get_named_arg::<Key>("account");
    let index = runtime::get_named_arg::<Option<U256>>("index");
    let ret: bool = ProviderDict::open().is_kyc_proved(account, index);
    runtime::ret(CLValue::from_t(ret).unwrap_or_revert());
}

/// Add a new entry to the list of KYC providers.
#[no_mangle]
pub extern "C" fn add_kyc_provider() {
    ProviderDict::open().add_kyc_provider(runtime::get_named_arg("provider"))
}

/// Declare a KYC provider contract as not acceptable.
/// This provider will not be asked in any of the following validity checks.
#[no_mangle]
pub extern "C" fn ban_provider() {
    ProviderDict::open().ban_provider(runtime::get_named_arg("provider"))
}

/// Restore a KYC provider contract's accessibility through the proxy.
/// This provider will be asked for an account's validity during checks.
#[no_mangle]
pub extern "C" fn unban_provider() {
    ProviderDict::open().unban_provider(runtime::get_named_arg("provider"))
}

#[no_mangle]
pub extern "C" fn call() {
    let (contract_package_hash, access_uref) = storage::create_contract_package_at_hash();
    let mut entry_points = EntryPoints::new();

    entry_points.add_entry_point(EntryPoint::new(
        "init",
        vec![Parameter::new(
            "initial_providers",
            CLType::List(Box::new(ContractPackageHash::cl_type())),
        )],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "is_kyc_proved",
        vec![],
        CLType::Bool,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "add_kyc_provider",
        vec![Parameter::new("provider", Key::cl_type())],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "ban_provider",
        vec![Parameter::new("provider", Key::cl_type())],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "unban_provider",
        vec![Parameter::new("provider", Key::cl_type())],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    let proxy_name: String = runtime::get_named_arg("name");

    let mut named_keys = NamedKeys::new();
    named_keys.insert(
        format!("{}-proxy_contract_package", proxy_name),
        storage::new_uref(contract_package_hash).into(),
    );
    let (contract_hash, _) =
        storage::add_contract_version(contract_package_hash, entry_points, named_keys);
    runtime::put_key(
        &format!("{}-proxy_package_hash", proxy_name),
        contract_package_hash.into(),
    );
    runtime::put_key(
        &format!("{}-proxy_contract", proxy_name),
        contract_hash.into(),
    );
    runtime::put_key(
        &format!("{}-proxy_access_token", proxy_name),
        access_uref.into(),
    );
    runtime::put_key(
        &format!("{}-proxy_contract_hash", proxy_name),
        storage::new_uref(contract_hash).into(),
    );

    let initial_providers =
        match runtime::get_named_arg::<Option<Vec<ContractPackageHash>>>("initial_providers") {
            Some(providers) => providers,
            None => Vec::new(),
        };

    call_versioned_contract(
        contract_package_hash,
        None,
        "init",
        runtime_args! {
            "initial_providers" => initial_providers
        },
    )
}

struct ProviderDict {
    uref: URef,
    len: u64,
}

impl ProviderDict {
    /// Create a new URef that represents a seed for a dictionary partition of the global state and puts it under the "kyc_providers" named key.
    /// It then stores initial_providers in the dictionary, while setting their status as validated.
    fn init(initial_providers: Vec<ContractPackageHash>) {
        let dict_uref = new_dictionary("kyc_providers").unwrap_or_revert();
        for (provider_index, provider_package_hash) in initial_providers.iter().enumerate() {
            dictionary_put(
                dict_uref,
                &provider_index.to_string(),
                *provider_package_hash,
            );
            dictionary_put(dict_uref, &provider_package_hash.to_string(), true);
        }
        dictionary_put(dict_uref, "len", initial_providers.len() as u64);
    }

    /// Return a struct consisting of the URef stored under "kyc_providers" named key that holds the corresponding dictionary,
    /// and the number of stored providers.
    fn open() -> Self {
        let uref = *runtime::get_key("kyc_providers")
            .unwrap_or_revert()
            .as_uref()
            .unwrap_or_revert();
        let len: u64 = dictionary_get(uref, "len")
            .unwrap_or_revert()
            .unwrap_or_revert();
        ProviderDict { uref, len }
    }

    /// Store a new KYC provider in the dictionary and set their status as validated.
    fn add_kyc_provider(&self, provider_key: Key) {
        let provider_package_hash = Self::convert_provider_key(provider_key);
        let str_provider = provider_package_hash.to_string();
        if dictionary_get::<bool>(self.uref, &str_provider)
            .unwrap_or_revert()
            .is_none()
        {
            dictionary_put(self.uref, &self.len.to_string(), provider_package_hash);
            dictionary_put(self.uref, &str_provider, true);
            dictionary_put(self.uref, "len", self.len + 1);
        }
    }

    /// Set a stored provider as invalid in the dictionary.
    fn ban_provider(&self, provider_key: Key) {
        let str_provider = Self::convert_provider_key(provider_key).to_string();
        if let Some(true) = dictionary_get::<bool>(self.uref, &str_provider).unwrap_or_revert() {
            dictionary_put(self.uref, &str_provider, false);
        }
    }

    /// Set a stored provider as validated in the dictionary.
    fn unban_provider(&self, provider_key: Key) {
        let str_provider = Self::convert_provider_key(provider_key).to_string();
        if let Some(false) = dictionary_get::<bool>(self.uref, &str_provider).unwrap_or_revert() {
            dictionary_put(self.uref, &str_provider, true);
        }
    }

    /// Convert provider `Key` to `ContractPackageHash`.
    fn convert_provider_key(provider_key: Key) -> ContractPackageHash {
        match provider_key {
            Key::Hash(provider_hash) => ContractPackageHash::from(provider_hash),
            _ => revert(ApiError::User(300)),
        }
    }

    /// Check account validity by asking all stored KYC providers sequentially, and return true on the first confirmation.
    /// If all available providers refused return false,
    fn is_kyc_proved(&self, account: Key, index: Option<U256>) -> bool {
        for provider_index in 0..=self.len {
            // check if there is a provider stored at the index
            if let Some(provider_package_hash) =
                dictionary_get::<ContractPackageHash>(self.uref, &provider_index.to_string())
                    .unwrap_or_revert()
            {
                // check whether the provider is banned (result is `false` bool)
                if let Some(true) =
                    dictionary_get::<bool>(self.uref, &provider_package_hash.to_string())
                        .unwrap_or_revert()
                {
                    // return with true on the first provider that says they have approved the account
                    if self.is_kyc_proved_single(provider_package_hash, account, index) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Call the is_kyc_proved entry point on a provider.
    fn is_kyc_proved_single(
        &self,
        provider_package_hash: ContractPackageHash,
        account: Key,
        index: Option<U256>,
    ) -> bool {
        call_versioned_contract(
            provider_package_hash,
            None,
            "is_kyc_proved",
            runtime_args! {
                "account" => account,
                "index" => index
            },
        )
    }
}
