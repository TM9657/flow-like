//! Sub-Catalog for Machine Learning
//!
//! This module contains various machine learning algorithms and dataset utilities based on the `[linfa]` crate.

use flow_like::flow::{
    execution::context::ExecutionContext,
    node::NodeLogic,
};
use flow_like_types::json;
use flow_like_types::{
    anyhow, create_id, json::json, sync::Mutex, Cacheable, Error, Ok, Result, Value
};
use linfa::{traits::Predict, DatasetBase};
use linfa::prelude::Pr;
use linfa_clustering::KMeans;
use linfa_linear::FittedLinearRegression;
use linfa_nn::distance::L2Dist;
use linfa_svm::Svm;
use ndarray::{Array1, Array2};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use std::collections::HashMap;
use linfa::composing::MultiClassModel;
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

#[derive(Debug, Serialize, Deserialize)]
struct ClassEntry {
    id: usize,
    name: String,
}

/// Helper-Module to serialize HashMap as Vec and deserialize Vec as HashMap for the class mappings.
/// For some reason, a pure HashMap<usize, _> attribute is not properly deserialized as usize keys are recognzed as strings only.
/// This causes the Load Model Node to fail (in a playground project it works, but here it doesn't).
/// A HashMap uize -> String is still useful though, as we can easily map class predictions to class names with this.
/// So that's why we are taking the detour writing our own serializer/deserializer for the classes attribute.
/// In the long run, we could consider writing our own dumper/loader logic to account for further customizations, 
/// e.g. serializing only strictly necessary information to reproduce a model to reduce checkpoint sizes.
mod vec_as_map {
    use super::ClassEntry;
    use serde::{Deserialize, Deserializer, Serializer};
    use serde::ser::SerializeSeq;
    use std::collections::HashMap;

    pub fn serialize<S>(
        map_opt: &Option<HashMap<usize, String>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match map_opt {
            Some(map) => {
                let mut seq = serializer.serialize_seq(Some(map.len()))?;
                for (id, name) in map {
                    let entry = ClassEntry { id: *id, name: name.clone() };
                    seq.serialize_element(&entry)?;
                }
                seq.end()
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Option<HashMap<usize, String>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt_vec: Option<Vec<ClassEntry>> = Option::deserialize(deserializer)?;
        Ok(opt_vec.map(|v| v.into_iter().map(|e| (e.id, e.name)).collect()))
    }
}

#[derive(Debug, Serialize, Deserialize)]
/// # Linfa models attached with additional metadata
pub struct ModelWithMeta<M> {
    pub model: M,
    /// Optional mapping from class index → class name
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "vec_as_map")]
    pub classes: Option<HashMap<usize, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
/// # Unified Type for Machine Learning Models from Linfa Crate
pub enum MLModel {
    KMeans(ModelWithMeta<KMeans<f64, L2Dist>>),
    SVMMultiClass(ModelWithMeta<Vec<(usize, Svm<f64, Pr>)>>),
    LinearRegression(ModelWithMeta<FittedLinearRegression<f64>>),
    //SVMClass(ModelWithMeta<Svm<f64, Pr>>),
    //PCA(ModelWithMeta<Pca<f64>>),
}

impl fmt::Display for MLModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MLModel::KMeans(_) => write!(f, "KMeans Clustering"),
            MLModel::LinearRegression(_) => write!(f, "Linear Regression"),
            MLModel::SVMMultiClass(_) => write!(f, "SVM Classification (Multiple Classes)"),
            //MLModel::SVMClass(_) => write!(f, "SVM Classification (Single Class)"),
            //MLModel::PCA(_) => write!(f, "PCA"),
        }
    }
}

impl MLModel {
    
    fn to_json_vec(&self) -> Result<Vec<u8>> {
        Ok(json::to_vec(&self)?)
    }
    
    fn predict_on_values(&self, values: &mut Vec<Value>, record_col: &str, target_col: &str) -> Result<()> {
        match self {
            MLModel::KMeans(model) => {
                let array = values_to_array2_f64(values, record_col)?;
                let dataset = DatasetBase::from(array);
                let predictions = model.model.predict(&dataset);
                for (value, pred) in values.iter_mut().zip(predictions.iter()) {
                    if let Value::Object(map) = value {
                        map.insert(target_col.to_string(), json!(pred));
                    }
                }
                Ok(())
            },
            MLModel::LinearRegression(model) => {
                let array = values_to_array2_f64(values, record_col)?;
                let dataset = DatasetBase::from(array);
                let predictions = model.model.predict(&dataset);
                for (value, pred) in values.iter_mut().zip(predictions.iter()) {
                    if let Value::Object(map) = value {
                        map.insert(target_col.to_string(), json!(pred));
                    }
                }
                Ok(())
            },
            MLModel::SVMMultiClass(model) => {
                let array = values_to_array2_f64(values, record_col)?;
                let dataset = DatasetBase::from(array);
                let mult_class = MultiClassModel::from_iter(model.model.clone());
                let predictions = mult_class.predict(&dataset);
                for (value, pred) in values.iter_mut().zip(predictions.iter()) {
                    if let Value::Object(map) = value {
                        if let Some(classes) = &model.classes {
                            let class = classes.get(pred).ok_or_else(|| anyhow!(format!("Couldn't map prediction {} to any of these classes {:?}", pred, classes)))?;
                            map.insert(target_col.to_string(), json!(class))
                        } else {
                            map.insert(target_col.to_string(), json!(pred))
                        };
                    }
                }
                Ok(())
            }
        }
    }

    fn predict_on_vector(&self, vector: Vec<f64>) -> Result<MLPrediction> {
        match self {
            MLModel::KMeans(model) => {
                let array = Array2::from_shape_vec((1, vector.len()), vector)?;
                let dataset = DatasetBase::from(array);
                let predictions = model.model.predict(&dataset);
                let score = *predictions
                    .first()
                    .ok_or_else(|| anyhow!("Got an empty prediction"))?;
                Ok(MLPrediction { score: score as f64, class: None } )
            },
            MLModel::LinearRegression(model) => {
                let array = Array2::from_shape_vec((1, vector.len()), vector)?;
                let dataset = DatasetBase::from(array);
                let predictions = model.model.predict(&dataset);
                let score = *predictions
                    .first()
                    .ok_or_else(|| anyhow!("Got an empty prediction"))?;
                Ok(MLPrediction { score, class: None } )
            },
            MLModel::SVMMultiClass(model) => {
                let array = Array2::from_shape_vec((1, vector.len()), vector)?;
                let dataset = DatasetBase::from(array);
                let mult_class = MultiClassModel::from_iter(model.model.clone());
                let predictions = mult_class.predict(&dataset);
                let score = *predictions
                    .first()
                    .ok_or_else(|| anyhow!("Got an empty prediction"))?;
                let class = if let Some(classes) = &model.classes {
                    Some(classes.get(&score).ok_or_else(|| anyhow!(format!("Couldn't map prediction {} to any of these classes {:?}", score, classes)))?)
                } else {
                    None
                };
                Ok(MLPrediction { score: score as f64, class: class.cloned() } )
            }
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct MLPrediction {
    pub score: f64,
    pub class: Option<String>
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
    for (r, value) in values.iter().enumerate() {
        let v = value.get(attr).ok_or_else(|| {
            anyhow!("Row {r}: expected object with key `{attr}`, got `{value:?}`")
        })?;
        flat.push(
            v.as_f64()
                .ok_or_else(|| anyhow!("Row {r}: failed to load as f64"))?,
        );
    }
    Ok(Array1::from(flat))
}

/// For a column `attr` in Vec<Values> attempt to load all rows as Array1<usize>
/// We are assuming that the col contains Strings which we map to unique ids
pub fn values_to_array1_usize(values: &[Value], attr: &str) -> Result<(Array1<usize>, HashMap<usize, String>)> {
    let mut flat = Vec::with_capacity(values.len());
    let mut name_to_id: HashMap<String, usize> = HashMap::new();
    let mut id_to_name: HashMap<usize, String> = HashMap::new();
    let mut next_id = 0;

    for (r, value) in values.iter().enumerate() {
        let v = value.get(attr).ok_or_else(|| {
            anyhow!("Row {r}: expected object with key `{attr}`, got `{value:?}`")
        })?;
        let s = v.as_str().ok_or_else(|| {
            anyhow!("Row {r}: failed to load `{attr}` as string, got `{v:?}`")
        })?;

        // assing new class id or reuse existing
        let id = *name_to_id.entry(s.to_string()).or_insert_with(|| {
            let id = next_id;
            next_id += 1;
            id_to_name.insert(id, s.to_string());
            id
        });

        flat.push(id);
    }
    Ok((Array1::from(flat), id_to_name))
}
