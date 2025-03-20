pub mod db;
pub mod path;

use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    let mut nodes = vec![
        Arc::new(Mutex::new(db::vector::CreateLocalDatabaseNode::default()))
            as Arc<Mutex<dyn NodeLogic>>,
        Arc::new(Mutex::new(
            db::vector::vector_search::VectorSearchLocalDatabaseNode::default(),
        )) as Arc<Mutex<dyn NodeLogic>>,
        Arc::new(Mutex::new(
            db::vector::insert::InsertLocalDatabaseNode::default(),
        )) as Arc<Mutex<dyn NodeLogic>>,
        Arc::new(Mutex::new(
            db::vector::insert::BatchInsertLocalDatabaseNode::default(),
        )) as Arc<Mutex<dyn NodeLogic>>,
        Arc::new(Mutex::new(
            db::vector::upsert::UpsertLocalDatabaseNode::default(),
        )) as Arc<Mutex<dyn NodeLogic>>,
        Arc::new(Mutex::new(
            db::vector::upsert::BatchUpsertLocalDatabaseNode::default(),
        )) as Arc<Mutex<dyn NodeLogic>>,
        Arc::new(Mutex::new(
            db::vector::purge::PurgeLocalDatabaseNode::default(),
        )) as Arc<Mutex<dyn NodeLogic>>,
        Arc::new(Mutex::new(
            db::vector::optimize::OptimizeLocalDatabaseNode::default(),
        )) as Arc<Mutex<dyn NodeLogic>>,
        Arc::new(Mutex::new(
            db::vector::list::ListLocalDatabaseNode::default(),
        )) as Arc<Mutex<dyn NodeLogic>>,
        Arc::new(Mutex::new(
            db::vector::index::IndexLocalDatabaseNode::default(),
        )) as Arc<Mutex<dyn NodeLogic>>,
        Arc::new(Mutex::new(
            db::vector::hybrid_search::HybridSearchLocalDatabaseNode::default(),
        )) as Arc<Mutex<dyn NodeLogic>>,
        Arc::new(Mutex::new(
            db::vector::fts_search::FTSLocalDatabaseNode::default(),
        )) as Arc<Mutex<dyn NodeLogic>>,
        Arc::new(Mutex::new(
            db::vector::filter::FilterLocalDatabaseNode::default(),
        )) as Arc<Mutex<dyn NodeLogic>>,
        Arc::new(Mutex::new(
            db::vector::delete::DeleteLocalDatabaseNode::default(),
        )) as Arc<Mutex<dyn NodeLogic>>,
        Arc::new(Mutex::new(
            db::vector::count::CountLocalDatabaseNode::default(),
        )) as Arc<Mutex<dyn NodeLogic>>,
    ];

    nodes.extend(path::register_functions().await);

    nodes
}
