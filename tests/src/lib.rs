use casper_engine_test_support::{
    DeployItemBuilder, ExecuteRequestBuilder, InMemoryWasmTestBuilder, ARG_AMOUNT,
    DEFAULT_ACCOUNT_ADDR, DEFAULT_PAYMENT, DEFAULT_RUN_GENESIS_REQUEST,
};
use casper_execution_engine::core::engine_state::ExecuteRequest;
use casper_types::{
    account::AccountHash, runtime_args, system::mint, ContractHash, ContractPackageHash, Key,
    PublicKey, RuntimeArgs, SecretKey, U512,
};
use rand::Rng;
use std::collections::BTreeMap;

pub struct ProxyContract {
    pub builder: InMemoryWasmTestBuilder,
    pub accounts: Vec<(PublicKey, AccountHash)>,
    pub package_hash: ContractPackageHash,
    pub contract_hash: ContractHash,
}

pub fn fund_account(account: &AccountHash) -> ExecuteRequest {
    let mut rng = rand::thread_rng();
    let deploy_item = DeployItemBuilder::new()
        .with_address(*DEFAULT_ACCOUNT_ADDR)
        .with_authorization_keys(&[*DEFAULT_ACCOUNT_ADDR])
        .with_empty_payment_bytes(runtime_args! {ARG_AMOUNT => *DEFAULT_PAYMENT})
        .with_transfer_args(runtime_args! {
            mint::ARG_AMOUNT => U512::from(30_000_000_000_000_u64),
            mint::ARG_TARGET => *account,
            mint::ARG_ID => <Option::<u64>>::None
        })
        .with_deploy_hash(rng.gen())
        .build();

    ExecuteRequestBuilder::from_deploy_item(deploy_item).build()
}

impl ProxyContract {
    pub fn deploy() -> Self {
        let mut rng = rand::thread_rng();
        let mut builder = InMemoryWasmTestBuilder::default();
        builder.run_genesis(&DEFAULT_RUN_GENESIS_REQUEST).commit();
        let mut accounts = Vec::new();
        for i in 0..3u8 {
            let secret_key: SecretKey = SecretKey::ed25519_from_bytes([i; 32]).unwrap();
            let public_key: PublicKey = (&secret_key).into();
            let account_hash = AccountHash::from(&public_key);
            accounts.push((public_key, account_hash));
            builder
                .exec(fund_account(&account_hash))
                .expect_success()
                .commit();
        }

        let mut deploy_builder = DeployItemBuilder::new()
            .with_empty_payment_bytes(runtime_args! {ARG_AMOUNT => *DEFAULT_PAYMENT})
            .with_address(accounts[0].1)
            .with_authorization_keys(&[accounts[0].1])
            .with_deploy_hash(rng.gen());

        deploy_builder = deploy_builder.with_session_code(
            "kyc-proxy.wasm",
            runtime_args! {
                "initial_providers" => Option::<Vec<ContractPackageHash>>::None
            },
        );
        let execute_request_builder =
            ExecuteRequestBuilder::from_deploy_item(deploy_builder.build());
        let exec = builder.exec(execute_request_builder.build());
        exec.expect_success().commit();

        let package_hash: ContractPackageHash = builder
            .query(
                None,
                Key::Account(accounts[0].1),
                &[
                    "kyc-proxy_contract_hash".to_string(),
                    "kyc-proxy_contract_package_hash".to_string(),
                ],
            )
            .expect("should be stored value.")
            .as_cl_value()
            .expect("should be cl value.")
            .clone()
            .into_t()
            .expect("Wrong type in query result.");

        let contract_hash: ContractHash = builder
            .query(
                None,
                Key::Account(accounts[0].1),
                &["kyc-proxy_contract_hash_wrapped".to_string()],
            )
            .expect("should be stored value.")
            .as_cl_value()
            .expect("should be cl value.")
            .clone()
            .into_t()
            .expect("Wrong type in query result.");

        ProxyContract {
            builder,
            accounts,
            package_hash,
            contract_hash,
        }
    }

    pub fn deploy_kyc(
        &mut self,
        deployer: AccountHash,
        kyc_name: &str,
    ) -> (ContractPackageHash, ContractHash) {
        let mut rng = rand::thread_rng();
        let mut deploy_builder = DeployItemBuilder::new()
            .with_empty_payment_bytes(runtime_args! {ARG_AMOUNT => *DEFAULT_PAYMENT})
            .with_address(deployer)
            .with_authorization_keys(&[deployer])
            .with_deploy_hash(rng.gen());

        let mut meta = BTreeMap::new();
        meta.insert("origin".to_string(), "kyc".to_string());

        deploy_builder = deploy_builder.with_session_code(
            "kyc-contract.wasm",
            runtime_args! {
                "name" => kyc_name,
                "contract_name" => "kyc",
                "symbol" => "symbol",
                "meta" => meta,
                "admin" => Key::Account(deployer)
            },
        );

        let execute_request_builder =
            ExecuteRequestBuilder::from_deploy_item(deploy_builder.build());
        let exec = self.builder.exec(execute_request_builder.build());
        exec.expect_success().commit();

        let package_hash: ContractPackageHash = self
            .builder
            .query(
                None,
                Key::Account(deployer),
                &[
                    "kyc_contract_hash".to_string(),
                    "kyc_contract_package_hash".to_string(),
                ],
            )
            .expect("should be stored value.")
            .as_cl_value()
            .expect("should be cl value.")
            .clone()
            .into_t()
            .expect("Wrong type in query result.");

        let contract_hash: ContractHash = self
            .builder
            .query(
                None,
                Key::Account(deployer),
                &["kyc_contract_hash_wrapped".to_string()],
            )
            .expect("should be stored value.")
            .as_cl_value()
            .expect("should be cl value.")
            .clone()
            .into_t()
            .expect("Wrong type in query result.");

        (package_hash, contract_hash)
    }

    fn call(&mut self, caller: AccountHash, method: &str, args: RuntimeArgs) {
        let mut rng = rand::thread_rng();
        let mut deploy_builder = DeployItemBuilder::new()
            .with_empty_payment_bytes(runtime_args! {ARG_AMOUNT => *DEFAULT_PAYMENT})
            .with_address(caller)
            .with_authorization_keys(&[caller])
            .with_deploy_hash(rng.gen());

        deploy_builder = deploy_builder.with_stored_session_hash(self.contract_hash, method, args);

        let execute_request_builder =
            ExecuteRequestBuilder::from_deploy_item(deploy_builder.build());
        let exec = self.builder.exec(execute_request_builder.build());
        exec.expect_success().commit();
    }

    pub fn add_kyc_provider(&mut self, provider_package_hash_key: ContractPackageHash) {
        self.call(
            self.accounts[0].1,
            "add_kyc_provider",
            runtime_args! {"provider"=>Key::Hash(provider_package_hash_key.value())},
        );
    }

    pub fn is_kyc_proved(&mut self, result: bool) {
        let mut rng = rand::thread_rng();
        let mut deploy_builder = DeployItemBuilder::new()
            .with_empty_payment_bytes(runtime_args! {ARG_AMOUNT => *DEFAULT_PAYMENT})
            .with_address(self.accounts[0].1)
            .with_authorization_keys(&[self.accounts[0].1])
            .with_deploy_hash(rng.gen());

        deploy_builder = deploy_builder.with_session_code(
            "test_contract.wasm",
            runtime_args! {
                "kyc-proxy_package_hash"=>self.package_hash,
                "result" => result
            },
        );

        let execute_request_builder =
            ExecuteRequestBuilder::from_deploy_item(deploy_builder.build());
        let exec = self.builder.exec(execute_request_builder.build());
        exec.expect_success().commit();
    }
}

#[test]
fn test_deploy() {
    ProxyContract::deploy();
}

#[test]
fn test_no_provider() {
    let mut proxy = ProxyContract::deploy();
    proxy.is_kyc_proved(false);
}

#[test]
#[should_panic = "User(999)"]
fn test_no_provider_failing() {
    let mut proxy = ProxyContract::deploy();
    proxy.is_kyc_proved(true);
}

#[test]
fn test_single_provider_proxy_negative() {
    let mut proxy = ProxyContract::deploy();
    let (first_provider_package_hash, _) = proxy.deploy_kyc(proxy.accounts[1].1, "first");
    proxy.add_kyc_provider(first_provider_package_hash);
    proxy.is_kyc_proved(false);
}
