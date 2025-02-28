use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod damerau_levenshtein;
pub mod hamming;
pub mod jaro;
pub mod jaro_winkler;
pub mod levenshtein;
pub mod optimal_string_alignment;
pub mod sorensen_dice;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    vec![
        Arc::new(Mutex::new(jaro_winkler::JaroWinklerDistanceNode::default())),
        Arc::new(Mutex::new(
            optimal_string_alignment::OptimalStringAlignmentDistanceNode::default(),
        )),
        Arc::new(Mutex::new(jaro::JaroDistanceNode::default())),
        Arc::new(Mutex::new(levenshtein::LevenshteinDistanceNode::default())),
        Arc::new(Mutex::new(
            damerau_levenshtein::DamerauLevenshteinDistanceNode::default(),
        )),
        Arc::new(Mutex::new(hamming::HammingDistanceNode::default())),
        Arc::new(Mutex::new(
            sorensen_dice::SorensenDiceCoefficientNode::default(),
        )),
    ]
}
