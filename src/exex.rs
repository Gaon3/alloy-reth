use reth_exex::ExExContext;
use reth_network_api::noop::NoopNetwork;
use reth_node_api::{FullNodeComponents, FullNodeTypes};

use crate::{RethLayer, RethLayerBuilder};

pub type RethLayerExex<Node> = RethLayer<
    <Node as FullNodeTypes>::Provider,
    <Node as FullNodeComponents>::Pool,
    NoopNetwork,
    <Node as FullNodeTypes>::Provider,
>;

pub fn new_layer_from_exex<Node: FullNodeComponents>(ctx: &ExExContext<Node>) -> RethLayerExex<Node> {
    let builder = RethLayerBuilder::default()
        .with_provider(ctx.provider.clone())
        .with_executor(ctx.task_executor.clone())
        .with_pool(ctx.pool.clone())
        .with_events(ctx.provider.clone())
        .with_noop_network();

    builder.into_layer()
}
