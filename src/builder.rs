use reth_evm_ethereum::EthEvmConfig;
use reth_network_api::{noop::NoopNetwork, NetworkInfo, Peers};
use reth_provider::{
    AccountReader, BlockReader, BlockReaderIdExt, CanonStateSubscriptions, ChainSpecProvider, ChangeSetReader,
    EvmEnvProvider, StateProviderFactory,
};
use reth_rpc_builder::{RethModuleRegistry, RpcModuleConfig};
use reth_tasks::TaskSpawner;
use reth_transaction_pool::{noop::NoopTransactionPool, TransactionPool};

use crate::RethLayer;

pub struct NoopCanonStateSubscriptions;

impl CanonStateSubscriptions for NoopCanonStateSubscriptions {
    fn subscribe_to_canonical_state(&self) -> reth_provider::CanonStateNotifications {
        let (_, receiver) = tokio::sync::broadcast::channel(0);
        receiver
    }
}

pub struct RethLayerBuilder<Reth, Pool, Net, Tasks, Events> {
    provider: Reth,
    pool: Pool,
    network: Net,
    executor: Tasks,
    events: Events,
}

impl Default for RethLayerBuilder<(), (), (), (), ()> {
    fn default() -> Self {
        Self { provider: (), pool: (), network: (), executor: (), events: () }
    }
}

impl<Reth, Pool, Net, Tasks, Events> RethLayerBuilder<Reth, Pool, Net, Tasks, Events> {
    pub fn new(provider: Reth, pool: Pool, network: Net, executor: Tasks, events: Events) -> Self {
        Self { provider, pool, network, executor, events }
    }

    pub fn with_provider<P>(self, provider: P) -> RethLayerBuilder<P, Pool, Net, Tasks, Events>
    where
        P: BlockReader + StateProviderFactory + EvmEnvProvider + 'static,
    {
        let Self { pool, network, executor, events, .. } = self;
        RethLayerBuilder { provider, pool, network, executor, events }
    }

    pub fn with_executor<T>(self, executor: T) -> RethLayerBuilder<Reth, Pool, Net, T, Events>
    where
        T: TaskSpawner + 'static,
    {
        let Self { pool, network, provider, events, .. } = self;
        RethLayerBuilder { provider, network, pool, executor, events }
    }

    pub fn with_pool<P>(self, pool: P) -> RethLayerBuilder<Reth, P, Net, Tasks, Events>
    where
        P: TransactionPool + 'static,
    {
        let Self { provider, network, executor, events, .. } = self;
        RethLayerBuilder { provider, network, pool, executor, events }
    }

    pub fn with_noop_pool(self) -> RethLayerBuilder<Reth, NoopTransactionPool, Net, Tasks, Events> {
        let Self { provider, executor, events, network, .. } = self;
        RethLayerBuilder { provider, executor, events, network, pool: NoopTransactionPool::default() }
    }

    pub fn with_network<N>(self, network: N) -> RethLayerBuilder<Reth, Pool, N, Tasks, Events>
    where
        N: NetworkInfo + Peers + 'static,
    {
        let Self { provider, pool, executor, events, .. } = self;
        RethLayerBuilder { provider, network, pool, executor, events }
    }

    pub fn with_noop_network(self) -> RethLayerBuilder<Reth, Pool, NoopNetwork, Tasks, Events> {
        let Self { provider, pool, executor, events, .. } = self;
        RethLayerBuilder { provider, pool, executor, events, network: NoopNetwork::default() }
    }

    pub fn with_events<E>(self, events: E) -> RethLayerBuilder<Reth, Pool, Net, Tasks, E>
    where
        E: CanonStateSubscriptions + 'static,
    {
        let Self { provider, pool, executor, network, .. } = self;
        RethLayerBuilder { provider, network, pool, executor, events }
    }

    pub fn with_noop_events(self) -> RethLayerBuilder<Reth, Pool, Net, Tasks, NoopCanonStateSubscriptions> {
        let Self { provider, pool, executor, network, .. } = self;
        RethLayerBuilder { provider, pool, executor, network, events: NoopCanonStateSubscriptions }
    }
}

impl<Reth, Pool, Net, Tasks, Events> RethLayerBuilder<Reth, Pool, Net, Tasks, Events>
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
    pub fn into_layer(self) -> RethLayer<Reth, Pool, Net, Events> {
        let config = RpcModuleConfig::default();
        let evm_config = EthEvmConfig::default();

        let mut builder = RethModuleRegistry::new(
            self.provider,
            self.pool,
            self.network,
            self.executor,
            self.events,
            config,
            evm_config,
        );

        RethLayer { eth: builder.eth_handlers() }
    }
}
