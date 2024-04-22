use std::marker::PhantomData;

use alloy_provider::{network::Ethereum, Provider, ProviderLayer};
use alloy_transport::Transport;

use reth_evm_ethereum::EthEvmConfig;
use reth_network_api::NetworkInfo;
use reth_provider::{
    BlockReader, BlockReaderIdExt, CanonStateSubscriptions, ChainSpecProvider, EvmEnvProvider, StateProviderFactory,
};
use reth_rpc_builder::EthHandlers;
use reth_transaction_pool::TransactionPool;

#[cfg(feature = "db")]
pub mod db;

#[cfg(feature = "exex")]
pub mod exex;

mod builder;
mod provider;

pub use builder::RethLayerBuilder;

#[derive(Debug)]
pub struct RethLayer<Reth, Pool, Net, Events> {
    eth: EthHandlers<Reth, Pool, Net, Events, EthEvmConfig>,
}

impl<Reth, Pool, Net, Events, P, T> ProviderLayer<P, T> for RethLayer<Reth, Pool, Net, Events>
where
    Reth: StateProviderFactory
        + BlockReader
        + EvmEnvProvider
        + BlockReaderIdExt
        + ChainSpecProvider
        + Clone
        + Unpin
        + 'static,
    Pool: TransactionPool + Clone + 'static,
    Net: NetworkInfo + Clone + 'static,
    Events: CanonStateSubscriptions + Clone + 'static,
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    type Provider = RethProvider<Reth, Pool, Net, Events, P, T>;

    fn layer(&self, inner: P) -> Self::Provider {
        RethProvider::new(self.eth.clone(), inner)
    }
}

#[derive(Debug, Clone)]
pub struct RethProvider<Reth, Pool, Net, Events, P, T>
where
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    eth: EthHandlers<Reth, Pool, Net, Events, EthEvmConfig>,
    inner: P,
    _pd: PhantomData<fn() -> (T, Ethereum)>,
}

impl<Reth, Pool, Net, Events, P, T> RethProvider<Reth, Pool, Net, Events, P, T>
where
    Reth: BlockReaderIdExt + ChainSpecProvider,
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    pub fn new(eth: EthHandlers<Reth, Pool, Net, Events, EthEvmConfig>, inner: P) -> Self {
        Self { eth, inner, _pd: PhantomData }
    }

    pub fn eth_handlers(&self) -> &EthHandlers<Reth, Pool, Net, Events, EthEvmConfig> {
        &self.eth
    }

    pub fn provider(&self) -> &Reth {
        self.eth.api.provider()
    }
}
