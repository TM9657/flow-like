use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub mod damerau_levenshtein;
pub mod hamming;
pub mod jaro;
pub mod jaro_winkler;
pub mod levenshtein;
pub mod optimal_string_alignment;
pub mod sorensen_dice;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(jaro_winkler::JaroWinklerDistanceNode::default()),
        Arc::new(optimal_string_alignment::OptimalStringAlignmentDistanceNode::default()),
        Arc::new(jaro::JaroDistanceNode::default()),
        Arc::new(levenshtein::LevenshteinDistanceNode::default()),
        Arc::new(damerau_levenshtein::DamerauLevenshteinDistanceNode::default()),
        Arc::new(hamming::HammingDistanceNode::default()),
        Arc::new(sorensen_dice::SorensenDiceCoefficientNode::default()),
    ]
}
