use std::sync::Arc;

use reth_blockchain_tree::noop::NoopBlockchainTree;
use reth_db::{mdbx::DatabaseArguments, models::client_version::ClientVersion, open_db_read_only, DatabaseEnv};
use reth_network_api::noop::NoopNetwork;
use reth_primitives::ChainSpecBuilder;
use reth_provider::{providers::BlockchainProvider, ProviderFactory};
use reth_tasks::TokioTaskExecutor;
use reth_transaction_pool::noop::NoopTransactionPool;

use crate::{RethLayer, RethProvider};

use super::NoopCanonStateSubscriptions;

pub type DBProvider = BlockchainProvider<Arc<DatabaseEnv>, NoopBlockchainTree>;

pub type RethDBLayer =
    RethLayer<DBProvider, NoopTransactionPool, NoopNetwork, TokioTaskExecutor, NoopCanonStateSubscriptions>;

pub type RethDBProvider<P, T> =
    RethProvider<DBProvider, NoopTransactionPool, NoopNetwork, NoopCanonStateSubscriptions, P, T>;

pub fn new_provider_from_db(db_path_env_var: &str) -> eyre::Result<DBProvider> {
    let db_path = std::env::var(db_path_env_var)?;
    let db_path = std::path::Path::new(db_path.as_str());

    let db =
        Arc::new(open_db_read_only(db_path.join("db").as_path(), DatabaseArguments::new(ClientVersion::default()))?);

    let chain_spec = Arc::new(ChainSpecBuilder::mainnet().build());

    let factory = ProviderFactory::new(db.clone(), chain_spec, db_path.join("static_files"))?;
    let provider = BlockchainProvider::new(factory, NoopBlockchainTree::default())?;
    Ok(provider)
}

pub fn new_layer_from_db(db_path_env_var: &str) -> eyre::Result<RethDBLayer> {
    let provider = new_provider_from_db(db_path_env_var)?;
    let layer = RethLayer::default()
        .with_provider(provider)
        .with_noop_pool()
        .with_noop_events()
        .with_noop_network()
        .with_executor(TokioTaskExecutor::default());
    Ok(layer)
}
