#![no_main]
#![no_std]

extern crate alloc;

use alloc::{format, string::String};
use casper_contract::contract_api::{
    runtime::{self},
    storage::{self},
};
use casper_types::{contracts::NamedKeys, EntryPoints};

#[no_mangle]
fn call() {
    // Read arguments for the constructor call.
    let (contract_package_hash, _) = storage::create_contract_package_at_hash();
    // let name: String = runtime::get_named_arg("name");
    // let symbol: String = runtime::get_named_arg("symbol");
    // let meta: Meta = runtime::get_named_arg("meta");
    let contract_name: String = runtime::get_named_arg("contract_name");
    // let admin: Key = runtime::get_named_arg("admin");

    let mut named_keys = NamedKeys::new();
    named_keys.insert(
        format!("{}_contract_package_hash", contract_name),
        storage::new_uref(contract_package_hash).into(),
    );

    let (contract_hash, _) =
        storage::add_contract_version(contract_package_hash, get_entry_points(), named_keys);

    runtime::put_key(
        &format!("{}_contract_hash", contract_name),
        contract_hash.into(),
    );
    runtime::put_key(
        &format!("{}_contract_hash_wrapped", contract_name),
        storage::new_uref(contract_hash).into(),
    );
}

fn get_entry_points() -> EntryPoints {
    EntryPoints::new()
}
