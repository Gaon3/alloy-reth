use std::{marker::PhantomData, sync::Arc};

use alloy_primitives::{Address, Bytes, StorageValue, U256};
use alloy_provider::{network::Ethereum, Network, Provider, RootProvider};
use alloy_rpc_types::{state::StateOverride, Block, BlockId, Bundle, Filter, Log};
use alloy_transport::{Transport, TransportErrorKind, TransportResult};
use async_trait::async_trait;
use reth_evm_ethereum::EthEvmConfig;
use reth_network_api::NetworkInfo;
use reth_primitives::BlockHash;
use reth_provider::{
    BlockIdReader, BlockReader, BlockReaderIdExt, CanonStateSubscriptions, ChainSpecProvider, EvmEnvProvider,
    HeaderProvider, StateProviderFactory,
};
use reth_rpc::{
    eth::{cache::EthStateCache, revm_utils::EvmOverrides},
    EthApi, EthFilter, EthPubSub,
};
use reth_rpc_api::{EthApiServer, EthFilterApiServer};
use reth_rpc_builder::EthHandlers;
use reth_tasks::pool::BlockingTaskPool;
use reth_transaction_pool::TransactionPool;

/// A provider which directly connects to the Reth database by instanciating the server
/// which handles RPC requests (normally running inside the node) on the client side.
/// Requests are directly handled by [`EthHandlers`], bypassing JSON-RPC
/// and allowing for better performances.
#[derive(Debug, Clone)]
pub struct RethProvider<Reth, Pool, Net, Events, P, T>
where
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    eth: Arc<EthHandlers<Reth, Pool, Net, Events, EthEvmConfig>>,
    inner: P,
    _pd: PhantomData<fn() -> (T, Ethereum)>,
}

impl<Reth, Pool, Net, Events, P, T> RethProvider<Reth, Pool, Net, Events, P, T>
where
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    /// Creates a new `RethProvider` with the given inner provider and `EthHandlers` instance.
    pub fn new(eth: Arc<EthHandlers<Reth, Pool, Net, Events, EthEvmConfig>>, inner: P) -> Self {
        Self { eth, inner, _pd: PhantomData }
    }

    /// Returns a reference to `EthHandlers`.
    pub fn eth(&self) -> &EthHandlers<Reth, Pool, Net, Events, EthEvmConfig> {
        &self.eth
    }

    pub fn eth_api(&self) -> &EthApi<Reth, Pool, Net, EthEvmConfig> {
        &self.eth.api
    }

    pub fn eth_cache(&self) -> &EthStateCache {
        &self.eth.cache
    }

    pub fn eth_filter(&self) -> &EthFilter<Reth, Pool> {
        &self.eth.filter
    }

    pub fn eth_pubsub(&self) -> &EthPubSub<Reth, Pool, Events, Net> {
        &self.eth.pubsub
    }

    pub fn eth_blocking_task_pool(&self) -> &BlockingTaskPool {
        &self.eth.blocking_task_pool
    }
}

impl<Reth, Pool, Net, Events, P, T> RethProvider<Reth, Pool, Net, Events, P, T>
where
    Reth: BlockReaderIdExt + ChainSpecProvider,
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    pub fn provider(&self) -> &Reth {
        self.eth.api.provider()
    }

    pub fn network(&self) -> &Net {
        self.eth.api.network()
    }

    pub fn pool(&self) -> &Pool {
        self.eth.api.pool()
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<Reth, Pool, Net, Events, P, T> Provider<T, Ethereum> for RethProvider<Reth, Pool, Net, Events, P, T>
where
    Reth: BlockReader
        + BlockIdReader
        + BlockReaderIdExt
        + ChainSpecProvider
        + HeaderProvider
        + StateProviderFactory
        + EvmEnvProvider
        + 'static,
    Pool: TransactionPool + 'static,
    Net: NetworkInfo + 'static,
    Events: CanonStateSubscriptions + 'static,
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    fn root(&self) -> &RootProvider<T, Ethereum> {
        self.inner.root()
    }

    async fn get_storage_at(&self, address: Address, key: U256, tag: BlockId) -> TransportResult<StorageValue> {
        let storage_value = self
            .eth
            .api
            .storage_at(address, key.into(), Some(tag))
            .await
            .map_err(|e| TransportErrorKind::Custom(Box::new(e)))?;
        Ok(U256::from_be_bytes(storage_value.into()))
    }

    async fn get_balance(&self, address: Address, tag: BlockId) -> TransportResult<U256> {
        Ok(self.eth.api.balance(address, Some(tag)).await.map_err(|e| TransportErrorKind::Custom(Box::new(e)))?)
    }

    async fn get_block(&self, id: BlockId, full: bool) -> TransportResult<Option<Block>> {
        let block = match id {
            BlockId::Hash(hash) => self.eth.api.block_by_hash(hash.into(), full).await,
            BlockId::Number(number) => self.eth.api.block_by_number(number, full).await,
        }
        .map_err(|e| TransportErrorKind::Custom(Box::new(e)))?
        .map(|block| block.inner);
        Ok(block)
    }

    async fn get_block_by_hash(&self, hash: BlockHash, full: bool) -> TransportResult<Option<Block>> {
        let block = self
            .eth
            .api
            .block_by_hash(hash, full)
            .await
            .map_err(|e| TransportErrorKind::Custom(Box::new(e)))?
            .map(|block| block.inner);
        Ok(block)
    }

    async fn get_code_at(&self, address: Address, tag: BlockId) -> TransportResult<Bytes> {
        Ok(self.eth.api.get_code(address, Some(tag)).await.map_err(|e| TransportErrorKind::Custom(Box::new(e)))?)
    }

    async fn get_logs(&self, filter: &Filter) -> TransportResult<Vec<Log>> {
        Ok(self.eth.filter.logs(filter.to_owned()).await.map_err(|e| TransportErrorKind::Custom(Box::new(e)))?)
    }

    async fn call(&self, tx: &<Ethereum as Network>::TransactionRequest, block: BlockId) -> TransportResult<Bytes> {
        let result = self
            .eth
            .api
            .call(tx.clone(), Some(block), Default::default())
            .await
            .map_err(|e| TransportErrorKind::Custom(Box::new(e)))?;
        Ok(result)
    }

    async fn call_with_overrides(
        &self,
        tx: &<Ethereum as Network>::TransactionRequest,
        block: BlockId,
        state: StateOverride,
    ) -> TransportResult<Bytes> {
        let overrides = EvmOverrides::new(Some(state), None);
        let result = self
            .eth
            .api
            .call(tx.clone(), Some(block), overrides)
            .await
            .map_err(|e| TransportErrorKind::Custom(Box::new(e)))?;
        Ok(result)
    }
}

impl<Reth, Pool, Net, Events, P, T> RethProvider<Reth, Pool, Net, Events, P, T>
where
    Reth: BlockReader
        + BlockIdReader
        + BlockReaderIdExt
        + ChainSpecProvider
        + HeaderProvider
        + StateProviderFactory
        + EvmEnvProvider
        + 'static,
    Pool: TransactionPool + 'static,
    Net: NetworkInfo + 'static,
    Events: CanonStateSubscriptions + 'static,
    P: Provider<T, Ethereum>,
    T: Transport + Clone,
{
    pub async fn call_many(
        &self,
        txs: &[<Ethereum as Network>::TransactionRequest],
    ) -> TransportResult<Vec<Result<Bytes, String>>> {
        let bundle = Bundle { transactions: txs.to_vec(), block_override: None };
        let results =
            self.eth.api.call_many(bundle, None, None).await.map_err(|e| TransportErrorKind::Custom(Box::new(e)))?;

        Ok(results.into_iter().map(|res| res.ensure_ok()).collect())
    }
}
