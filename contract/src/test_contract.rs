#![no_main]
#![no_std]

use casper_contract::contract_api::runtime::{
    call_versioned_contract, get_caller, get_named_arg, revert,
};
use casper_types::{runtime_args, ApiError, Key, RuntimeArgs, U256};

// Since the `is_kyc_proved` entry point on the contract returns data, it can only be called from,
// another contract, or session code. This necessitates the usage of this session code in the tests.
#[no_mangle]
pub extern "C" fn call() {
    if get_named_arg::<bool>("result")
        != call_versioned_contract::<bool>(
            get_named_arg("kyc_proxy_package_hash"),
            None,
            "is_kyc_proved",
            runtime_args! {
                "account" => Key::Account(get_caller()),
                "index" => Option::<U256>::None
            },
        )
    {
        revert(ApiError::User(999))
    }
}
