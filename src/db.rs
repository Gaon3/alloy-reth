use std::sync::Arc;

use reth_blockchain_tree::noop::NoopBlockchainTree;
use reth_db::{mdbx::DatabaseArguments, models::client_version::ClientVersion, open_db_read_only, DatabaseEnv};
use reth_primitives::ChainSpecBuilder;
use reth_provider::{providers::BlockchainProvider, ProviderFactory};

pub fn new_provider_from_db(
    db_path_env_var: &str,
) -> eyre::Result<BlockchainProvider<Arc<DatabaseEnv>, NoopBlockchainTree>> {
    let db_path = std::env::var(db_path_env_var)?;
    let db_path = std::path::Path::new(db_path.as_str());

    let db =
        Arc::new(open_db_read_only(db_path.join("db").as_path(), DatabaseArguments::new(ClientVersion::default()))?);

    let chain_spec = Arc::new(ChainSpecBuilder::mainnet().build());

    let factory = ProviderFactory::new(db.clone(), chain_spec, db_path.join("static_files"))?;
    let provider = BlockchainProvider::new(factory, NoopBlockchainTree::default())?;

    Ok(provider)
}
