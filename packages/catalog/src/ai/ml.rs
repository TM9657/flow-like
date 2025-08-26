/// # Machine Learning Nodes
use flow_like::flow::{execution::context::ExecutionContext, node::NodeLogic};
use flow_like_types::{Cacheable, Error, Result, Value, anyhow, create_id, sync::Mutex};
use linfa::DatasetBase;
use linfa::prelude::Pr;
use linfa_clustering::KMeans;
use linfa_linear::FittedLinearRegression;
use linfa_nn::distance::L2Dist;
use linfa_reduction::Pca;
use linfa_svm::Svm;
use ndarray::{Array2, ArrayBase, Dim, OwnedRepr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
pub mod classification;
pub mod clustering;
pub mod dataset;
pub mod reduction;
pub mod load;
pub mod prediction;
pub mod save;

/// Add Machine Learning Nodes to Catalog Lib
pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let nodes: Vec<Arc<dyn NodeLogic>> = vec![
        Arc::new(clustering::kmeans::FitKMeansNode::default()),
        Arc::new(prediction::MLPredictNode::default()),
        Arc::new(save::SaveMLModelNode::default()),
        Arc::new(load::LoadMLModelNode::default()),
    ];
    nodes
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
/// Available Machine Learning Models from Linfa Crate
pub enum MLModel {
    KMeans(KMeans<f64, L2Dist>),
    SVMClass(Svm<f64, Pr>),
    SVMMultiClass(Vec<(usize, Svm<f64, Pr>)>),
    LinearRegression(FittedLinearRegression<f64>),
    PCA(Pca<f64>),
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

/// Utility: Load LanceDB records (column of vectors) as ndarray
pub fn values_to_array(values: &[Value], col: &str) -> Result<Array2<f64>, Error> {
    // Determine dimensions
    let rows = values.len();
    let cols = values
        .get(0)
        .and_then(|obj| obj.get(col))
        .and_then(|v| v.as_array())
        .map(|arr| arr.len())
        .ok_or_else(|| anyhow!("Missing or invalid 'vector' in first element"))?;

    // Preallocate flat storage
    let mut flat = Vec::with_capacity(rows * cols);

    for (i, obj) in values.iter().enumerate() {
        let arr = obj
            .get(col)
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("Row {i} missing 'vector'"))?;

        if arr.len() != cols {
            return Err(anyhow!(
                "Row {i} has inconsistent length (expected {cols}, got {})",
                arr.len()
            ));
        }

        for (j, x) in arr.iter().enumerate() {
            flat.push(
                x.as_f64()
                    .ok_or_else(|| anyhow!("Invalid f64 at row {i}, col {j}"))?,
            );
        }
    }

    Ok(Array2::from_shape_vec((rows, cols), flat)?)
}

/// Utility: Load LanceDB records (column of vectors) as Linfa Database
pub fn values_to_dataset(
    values: &[Value],
    col: &str,
) -> Result<
    DatasetBase<
        ArrayBase<OwnedRepr<f64>, Dim<[usize; 2]>>,
        ArrayBase<OwnedRepr<()>, Dim<[usize; 1]>>,
    >,
    Error,
> {
    let array = values_to_array(values, col)?;
    let ds = DatasetBase::from(array);
    Ok(ds)
}

pub async fn load_dataset_from_db() -> Result<(), Error> {
    Ok(())
}

pub fn load_dataset_from_csv() -> Result<(), Error> {
    Ok(())
}
