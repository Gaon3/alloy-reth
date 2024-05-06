use reth_exex::ExExContext;
use reth_network_api::noop::NoopNetwork;
use reth_node_api::{FullNodeComponents, FullNodeTypes};
use reth_tasks::TaskExecutor;

use crate::{RethLayer, RethProvider};

pub type RethExExLayer<Node> = RethLayer<
    <Node as FullNodeTypes>::Provider,
    <Node as FullNodeComponents>::Pool,
    NoopNetwork,
    TaskExecutor,
    <Node as FullNodeTypes>::Provider,
>;

pub type RethExExProvider<Node, P, T> = RethProvider<
    <Node as FullNodeTypes>::Provider,
    <Node as FullNodeComponents>::Pool,
    NoopNetwork,
    <Node as FullNodeTypes>::Provider,
    P,
    T,
>;

pub fn new_layer_from_exex<Node: FullNodeComponents>(ctx: &ExExContext<Node>) -> RethExExLayer<Node> {
    RethLayer::default()
        .with_provider(ctx.provider.clone())
        .with_executor(ctx.task_executor.clone())
        .with_pool(ctx.pool.clone())
        .with_events(ctx.provider.clone())
        .with_noop_network()
}
