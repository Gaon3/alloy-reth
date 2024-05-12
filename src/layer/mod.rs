use std::sync::{Arc, OnceLock};

use alloy_provider::{network::Ethereum, Provider, ProviderLayer};
use alloy_transport::Transport;
use reth_evm_ethereum::EthEvmConfig;
use reth_network_api::{noop::NoopNetwork, NetworkInfo, Peers};
use reth_provider::{
    AccountReader, BlockReader, BlockReaderIdExt, CanonStateSubscriptions, ChainSpecProvider, ChangeSetReader,
    EvmEnvProvider, StateProviderFactory,
};
use reth_rpc_builder::{EthHandlers, RethModuleRegistry, RpcModuleConfig};

use reth_tasks::TaskSpawner;
use reth_transaction_pool::{noop::NoopTransactionPool, TransactionPool};

use crate::RethProvider;

#[derive(Clone)]
pub struct NoopCanonStateSubscriptions;

impl CanonStateSubscriptions for NoopCanonStateSubscriptions {
    fn subscribe_to_canonical_state(&self) -> reth_provider::CanonStateNotifications {
        let (_, receiver) = tokio::sync::broadcast::channel(0);
        receiver
    }
}

#[cfg(feature = "db")]
pub mod db;

#[cfg(feature = "exex")]
pub mod exex;

#[derive(Debug)]
pub struct RethLayer<Reth, Pool, Net, Tasks, Events> {
    provider: Reth,
    pool: Pool,
    network: Net,
    executor: Tasks,
    events: Events,
    #[allow(clippy::type_complexity)]
    eth: OnceLock<Arc<EthHandlers<Reth, Pool, Net, Events, EthEvmConfig>>>,
}

impl<Reth, Pool, Net, Tasks, Events> RethLayer<Reth, Pool, Net, Tasks, Events>
where
    Reth: BlockReaderIdExt
        + AccountReader
        + StateProviderFactory
        + EvmEnvProvider
        + ChainSpecProvider
        + ChangeSetReader
        + Clone
        + Unpin
        + 'static,
    Pool: TransactionPool + Clone + 'static,
    Net: NetworkInfo + Peers + Clone + 'static,
    Tasks: TaskSpawner + Clone + 'static,
    Events: CanonStateSubscriptions + Clone + 'static,
{
    pub fn eth_handlers(&self) -> &Arc<EthHandlers<Reth, Pool, Net, Events, EthEvmConfig>> {
        self.eth.get_or_init(|| {
            let config = RpcModuleConfig::default();
            let evm_config = EthEvmConfig::default();

            // It is ok to clone instead of move because these are all Arcs.
            let mut builder = RethModuleRegistry::new(
                self.provider.clone(),
                self.pool.clone(),
                self.network.clone(),
                self.executor.clone(),
                self.events.clone(),
                config,
                evm_config,
            );

            Arc::new(builder.eth_handlers())
        })
    }
}

impl<Reth, Pool, Net, Tasks, Events, P, T> ProviderLayer<P, T> for RethLayer<Reth, Pool, Net, Tasks, Events>
where
    Reth: StateProviderFactory
        + BlockReader
        + EvmEnvProvider
        + BlockReaderIdExt
        + AccountReader
        + ChainSpecProvider
        + ChangeSetReader
        + Clone
        + Unpin
        + 'static,
    Pool: TransactionPool + Clone + 'static,
    Net: NetworkInfo + Peers + Clone + 'static,
    Tasks: TaskSpawner + Clone + 'static,
    Events: CanonStateSubscriptions + Clone + 'static,
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    type Provider = RethProvider<Reth, Pool, Net, Events, P, T>;

    fn layer(&self, inner: P) -> Self::Provider {
        let eth = self.eth_handlers();
        RethProvider::new(eth.clone(), inner)
    }
}

impl Default for RethLayer<(), (), (), (), ()> {
    fn default() -> Self {
        Self { provider: (), pool: (), network: (), executor: (), events: (), eth: OnceLock::new() }
    }
}

impl<Reth, Pool, Net, Tasks, Events> RethLayer<Reth, Pool, Net, Tasks, Events> {
    pub fn new(provider: Reth, pool: Pool, network: Net, executor: Tasks, events: Events) -> Self {
        Self { provider, pool, network, executor, events, eth: OnceLock::new() }
    }

    pub fn with_provider<P>(self, provider: P) -> RethLayer<P, Pool, Net, Tasks, Events>
    where
        P: BlockReader + StateProviderFactory + EvmEnvProvider + 'static,
    {
        let Self { pool, network, executor, events, .. } = self;
        RethLayer { provider, pool, network, executor, events, eth: OnceLock::new() }
    }

    pub fn with_executor<T>(self, executor: T) -> RethLayer<Reth, Pool, Net, T, Events>
    where
        T: TaskSpawner + 'static,
    {
        let Self { pool, network, provider, events, .. } = self;
        RethLayer { provider, network, pool, executor, events, eth: OnceLock::new() }
    }

    pub fn with_pool<P>(self, pool: P) -> RethLayer<Reth, P, Net, Tasks, Events>
    where
        P: TransactionPool + 'static,
    {
        let Self { provider, network, executor, events, .. } = self;
        RethLayer { provider, network, pool, executor, events, eth: OnceLock::new() }
    }

    pub fn with_noop_pool(self) -> RethLayer<Reth, NoopTransactionPool, Net, Tasks, Events> {
        let Self { provider, executor, events, network, .. } = self;
        RethLayer { provider, executor, events, network, pool: NoopTransactionPool::default(), eth: OnceLock::new() }
    }

    pub fn with_network<N>(self, network: N) -> RethLayer<Reth, Pool, N, Tasks, Events>
    where
        N: NetworkInfo + Peers + 'static,
    {
        let Self { provider, pool, executor, events, .. } = self;
        RethLayer { provider, network, pool, executor, events, eth: OnceLock::new() }
    }

    pub fn with_noop_network(self) -> RethLayer<Reth, Pool, NoopNetwork, Tasks, Events> {
        let Self { provider, pool, executor, events, .. } = self;
        RethLayer { provider, pool, executor, events, network: NoopNetwork::default(), eth: OnceLock::new() }
    }

    pub fn with_events<E>(self, events: E) -> RethLayer<Reth, Pool, Net, Tasks, E>
    where
        E: CanonStateSubscriptions + 'static,
    {
        let Self { provider, pool, executor, network, .. } = self;
        RethLayer { provider, network, pool, executor, events, eth: OnceLock::new() }
    }

    pub fn with_noop_events(self) -> RethLayer<Reth, Pool, Net, Tasks, NoopCanonStateSubscriptions> {
        let Self { provider, pool, executor, network, .. } = self;
        RethLayer { provider, pool, executor, network, events: NoopCanonStateSubscriptions, eth: OnceLock::new() }
    }
}
