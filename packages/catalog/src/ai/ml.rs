//! Sub-Catalog for Machine Learning
//!
//! This module contains various machine learning algorithms and dataset utilities based on the `[linfa]` crate.

use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
};
use flow_like_types::{
    Cacheable, Error, Result, Value, anyhow, create_id, json::json, sync::Mutex,
};
use linfa::DatasetBase;
use linfa::prelude::Pr;
use linfa_clustering::KMeans;
use linfa_linear::FittedLinearRegression;
use linfa_nn::distance::L2Dist;
use linfa_reduction::Pca;
use linfa_svm::Svm;
use ndarray::{Array1, Array2, ArrayBase, Dim, OwnedRepr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
pub mod classification;
pub mod clustering;
pub mod dataset;
pub mod load;
pub mod prediction;
pub mod reduction;
pub mod regression;
pub mod save;

/// Max number of records for train/prediction
/// TODO: block-wise processing, at least for predictions
pub const MAX_RECORDS: usize = 10000;

/// Add Machine Learning Nodes to Catalog Lib
pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let nodes: Vec<Arc<dyn NodeLogic>> = vec![
        Arc::new(clustering::kmeans::FitKMeansNode::default()),
        Arc::new(classification::svm::FitSVMMultiClassNode::default()),
        Arc::new(regression::linear::FitLinearRegressionNode::default()),
        Arc::new(prediction::MLPredictNode::default()),
        Arc::new(save::SaveMLModelNode::default()),
        Arc::new(load::LoadMLModelNode::default()),
    ];
    nodes
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
/// # Unified Type for Machine Learning Models from Linfa Crate
/// Untagged feature flag allows to auto-deserialize as the correct variant
pub enum MLModel {
    KMeans(KMeans<f64, L2Dist>),
    SVMClass(Svm<f64, Pr>),
    SVMMultiClass(Vec<(usize, Svm<f64, Pr>)>),
    LinearRegression(FittedLinearRegression<f64>),
    PCA(Pca<f64>),
}

/// # Unified Type for Machine Learning Datasets from Linfa Crate
pub enum MLDataset {
    Unlabeled(
        DatasetBase<
            ArrayBase<OwnedRepr<f64>, Dim<[usize; 2]>>,
            ArrayBase<OwnedRepr<()>, Dim<[usize; 1]>>,
        >,
    ),
    Classification(
        DatasetBase<
            ArrayBase<OwnedRepr<f64>, Dim<[usize; 2]>>,
            ArrayBase<OwnedRepr<usize>, Dim<[usize; 1]>>,
        >,
    ),
    Regression(
        DatasetBase<
            ArrayBase<OwnedRepr<f64>, Dim<[usize; 2]>>,
            ArrayBase<OwnedRepr<f64>, Dim<[usize; 1]>>,
        >,
    ),
}

pub enum MLTargetType {
    Numerical,
    Categorical,
}

impl fmt::Display for MLModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MLModel::KMeans(_) => write!(f, "KMeans Clustering"),
            MLModel::LinearRegression(_) => write!(f, "Linear Regression"),
            MLModel::SVMClass(_) => write!(f, "SVM Classification (Single Class)"),
            MLModel::SVMMultiClass(_) => write!(f, "SVM Classification (Multiple Classes)"),
            MLModel::PCA(_) => write!(f, "PCA"),
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
/// Unified Machine Learning Model Type on Board Level
pub struct NodeMLModel {
    pub model_ref: String,
}

pub struct NodeMLModelWrapper {
    pub model: Arc<Mutex<MLModel>>,
}

impl Cacheable for NodeMLModelWrapper {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeMLModel {
    pub async fn new(ctx: &mut ExecutionContext, model: MLModel) -> Self {
        let id = create_id();
        let model_ref = Arc::new(Mutex::new(model));
        let wrapper = NodeMLModelWrapper {
            model: model_ref.clone(),
        };
        ctx.cache
            .write()
            .await
            .insert(id.clone(), Arc::new(wrapper));
        NodeMLModel { model_ref: id }
    }

    pub async fn get_model(&self, ctx: &mut ExecutionContext) -> Result<Arc<Mutex<MLModel>>> {
        let model = ctx
            .cache
            .read()
            .await
            .get(&self.model_ref)
            .cloned()
            .ok_or_else(|| flow_like_types::anyhow!("MLModel not found in cache!"))?;
        let model_wrapper = model
            .as_any()
            .downcast_ref::<NodeMLModelWrapper>()
            .ok_or_else(|| flow_like_types::anyhow!("Could not downcast to NodeMLModelWrapper"))?;
        let model = model_wrapper.model.clone();
        Ok(model)
    }
}

// -----------------------------------
// Utility fns to map Lance Vec<Values> to ndarrays
// TODO: can we merge these using generic types to avoid code duplication for identical behavior?
// -----------------------------------

/// For a column `attr` in Vec<Values> attempt to load all rows as Array2<f64> assuming that `attr` is a FixedSizeList of Vec<f64>
pub fn values_to_array2_f64(values: &[Value], attr: &str) -> Result<Array2<f64>, Error> {
    // Determine dimensions
    let rows = values.len();
    let cols = values
        .get(0)
        .and_then(|value| value.get(attr))
        .and_then(|v| v.as_array())
        .map(|arr| arr.len())
        .ok_or_else(|| anyhow!("Row 0: expected object with key `{attr}`"))?;

    // Preallocate flat storage
    let mut flat = Vec::with_capacity(rows * cols);

    for (i, value) in values.iter().enumerate() {
        let arr = value.get(attr).and_then(|v| v.as_array()).ok_or_else(|| {
            anyhow!("Row {i}: expected object with key `{attr}`, got `{value:?}`")
        })?;

        if arr.len() != cols {
            return Err(anyhow!(
                "Row {i}: inconsistent length (expected {cols}, got {})",
                arr.len()
            ));
        }

        for (j, x) in arr.iter().enumerate() {
            flat.push(
                x.as_f64()
                    .ok_or_else(|| anyhow!("Row {i}, col {j}: failed to load as f64"))?,
            );
        }
    }
    Ok(Array2::from_shape_vec((rows, cols), flat)?)
}

/// For a column `attr` in Vec<Values> attempt to load all rows as Array1<f64>
pub fn values_to_array1_f64(values: &[Value], attr: &str) -> Result<Array1<f64>> {
    let mut flat = Vec::with_capacity(values.len());
    for (i, value) in values.iter().enumerate() {
        let v = value.get(attr).ok_or_else(|| {
            anyhow!("Row {i}: expected object with key `{attr}`, got `{value:?}`")
        })?;
        flat.push(
            v.as_f64()
                .ok_or_else(|| anyhow!("Row {i}: failed to load as f64"))?,
        );
    }
    Ok(Array1::from(flat))
}

/// For a column `attr` in Vec<Values> attempt to load all rows as Array1<usize>
pub fn values_to_array1_usize(values: &[Value], attr: &str) -> Result<Array1<usize>> {
    let mut flat = Vec::with_capacity(values.len());
    for (i, value) in values.iter().enumerate() {
        let v = value.get(attr).ok_or_else(|| {
            anyhow!("Row {i}: expected object with key `{attr}`, got `{value:?}`")
        })?;
        flat.push(
            v.as_u64()
                .and_then(|n| usize::try_from(n).ok()) // u64 → usize safely
                .ok_or_else(|| anyhow!("Row {i}: failed to load as usize"))?,
        );
    }
    Ok(Array1::from(flat))
}

/// Utility: Load LanceDB records (column of vectors) as Linfa Database
/// Optional target col for supervised training
pub fn values_to_dataset(
    values: &[Value],
    train_col: &str,
    target_col: Option<&str>,
    target_format: Option<MLTargetType>,
) -> Result<MLDataset, Error> {
    let train_array = values_to_array2_f64(values, train_col)?;
    if let Some(target_col) = target_col {
        let target_format = target_format.ok_or(anyhow!("Target Format Not Set!"))?;
        match target_format {
            MLTargetType::Categorical => {
                let target_array = values_to_array1_usize(values, target_col)?;
                Ok(MLDataset::Classification(
                    DatasetBase::from(train_array).with_targets(target_array),
                ))
            }
            MLTargetType::Numerical => {
                let target_array = values_to_array1_f64(values, target_col)?;
                Ok(MLDataset::Regression(
                    DatasetBase::from(train_array).with_targets(target_array),
                ))
            }
        }
    } else {
        Ok(MLDataset::Unlabeled(DatasetBase::from(train_array)))
    }
}

/// Updates records with predictions by adding a new field.
///
/// # Arguments
/// * `records` - a vector of JSON records
/// * `predictions` - a 1D ndarray of `usize` predictions
/// * `attr_name` - the name of the new attribute to insert
///
/// # Returns
/// Updated vector of records with the new attribute.
fn update_records_with_predictions<T>(
    mut records: Vec<Value>,
    predictions: Array1<T>,
    attr_name: &str,
) -> Result<Vec<Value>, Error>
where
    T: Copy + Serialize,
{
    if records.len() != predictions.len() {
        return Err(anyhow!("Records and predictions have different lengths!"));
    }
    for (record, pred) in records.iter_mut().zip(predictions.iter()) {
        if let Value::Object(map) = record {
            map.insert(attr_name.to_string(), json!(pred));
        }
    }
    Ok(records)
}

/// Utility: Remove pin on update
fn remove_pin(node: &mut Node, name: &str) {
    if let Some(pin) = node.get_pin_by_name(name) {
        node.pins.remove(&pin.id.clone());
    }
}
