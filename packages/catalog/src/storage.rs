pub mod db;
pub mod path;

use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let mut nodes: Vec<Arc<dyn NodeLogic>> = vec![
        Arc::new(db::vector::CreateLocalDatabaseNode::default()),
        Arc::new(db::vector::vector_search::VectorSearchLocalDatabaseNode::default()),
        Arc::new(db::vector::insert::InsertLocalDatabaseNode::default()),
        Arc::new(db::vector::insert::BatchInsertLocalDatabaseNode::default()),
        Arc::new(db::vector::upsert::UpsertLocalDatabaseNode::default()),
        Arc::new(db::vector::upsert::BatchUpsertLocalDatabaseNode::default()),
        Arc::new(db::vector::purge::PurgeLocalDatabaseNode::default()),
        Arc::new(db::vector::optimize::OptimizeLocalDatabaseNode::default()),
        Arc::new(db::vector::list::ListLocalDatabaseNode::default()),
        Arc::new(db::vector::index::IndexLocalDatabaseNode::default()),
        Arc::new(db::vector::hybrid_search::HybridSearchLocalDatabaseNode::default()),
        Arc::new(db::vector::fts_search::FTSLocalDatabaseNode::default()),
        Arc::new(db::vector::filter::FilterLocalDatabaseNode::default()),
        Arc::new(db::vector::delete::DeleteLocalDatabaseNode::default()),
        Arc::new(db::vector::count::CountLocalDatabaseNode::default()),
        Arc::new(db::vector::schema::GetSchemaLocalDatabaseNode::default()),
    ];

    nodes.extend(path::register_functions().await);

    nodes
}
