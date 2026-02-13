---
title: Machine Learning
description: Train and deploy ML models for classification, regression, and clustering
sidebar:
  order: 4
---

Flow-Like includes a complete machine learning toolkit built on **linfa** (Rust's ML library) and **ONNX Runtime** for neural network inference. Train models visually without writing code.

## Available Algorithms

### Classification

| Algorithm | Node | Best For |
|-----------|------|----------|
| **Decision Tree** | Fit Decision Tree | Interpretable rules, multi-class |
| **Naive Bayes** | Fit Naive Bayes | Fast baseline, Gaussian features |
| **SVM** | Fit SVM Multi-Class | High accuracy, complex boundaries |

### Regression

| Algorithm | Node | Best For |
|-----------|------|----------|
| **Linear Regression** | Fit Linear Regression | Continuous predictions, feature importance |

### Clustering

| Algorithm | Node | Best For |
|-----------|------|----------|
| **K-Means** | Fit KMeans | Known cluster count, spherical clusters |
| **DBSCAN** | Fit DBSCAN | Unknown cluster count, outlier detection |

### Dimensionality Reduction

| Algorithm | Node | Best For |
|-----------|------|----------|
| **PCA** | Fit PCA | Feature reduction, visualization prep |

### Deep Learning (ONNX)

| Model Type | Node | Best For |
|------------|------|----------|
| **Image Classification** | ONNX TIMM | Classify images |
| **Object Detection** | ONNX YOLO/D-FINE | Detect objects in images |
| **Teachable Machine** | Teachable Machine | Quick prototyping |

## Data Preparation

### Input Format

ML nodes expect data in a **LanceDB database** with:
- A `records` column: 2D float array (feature matrix)
- A `targets` column: labels (classification) or values (regression)

### Preparing Your Data

```
┌────────────────────────────────────────────────────────────┐
│                                                            │
│  1. Load Data (CSV, SQL, etc.)                            │
│       │                                                    │
│       ▼                                                    │
│  2. Insert into Database                                   │
│       │                                                    │
│       ▼                                                    │
│  3. Format as records/targets                              │
│       │                                                    │
│       ▼                                                    │
│  4. Split (train/test)                                     │
│       │                                                    │
│       ▼                                                    │
│  5. Train Model                                            │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

### Dataset Splitting

**Random Split:**
```
Split Dataset
    │
    ├── Database: (input data)
    ├── Split Ratio: 0.8  (80% train, 20% test)
    │
    ├── Train ──▶ (training database)
    └── Test ──▶ (test database)
```

**Stratified Split** (preserves class distribution):
```
Stratified Split
    │
    ├── Database: (input data)
    ├── Target Column: "label"
    ├── Split Ratio: 0.8
    │
    ├── Train ──▶ (balanced training set)
    └── Test ──▶ (balanced test set)
```

:::tip[Use Stratified for Imbalanced Data]
If your classes are imbalanced (e.g., 95% vs 5%), always use stratified splitting to ensure both train and test sets have representative samples.
:::

### Other Data Operations

| Node | Purpose |
|------|---------|
| **Shuffle Dataset** | Randomize row order |
| **Sample Dataset** | Take a random subset |

## Classification Models

### Decision Tree

Decision trees create interpretable if-then rules:

```
Fit Decision Tree
    │
    ├── Database: (training data)
    ├── Max Depth: 10  (0 = unlimited)
    ├── Min Samples Split: 2
    │
    └── Model ──▶ (trained decision tree)
```

**When to use:**
- You need to explain predictions
- Data has clear decision boundaries
- Multi-class classification

**Parameters:**
| Parameter | Effect | Recommendation |
|-----------|--------|----------------|
| Max Depth | Tree complexity | Start with 5-10, increase if underfitting |
| Min Samples Split | Minimum samples to split | Higher values prevent overfitting |

### Naive Bayes

Fast Gaussian classifier:

```
Fit Naive Bayes
    │
    ├── Database: (training data)
    │
    └── Model ──▶ (trained Naive Bayes)
```

**When to use:**
- Quick baseline model
- Features are roughly Gaussian
- Fast inference needed

**Pros/Cons:**
| Pros | Cons |
|------|------|
| Very fast training | Assumes feature independence |
| Works with small datasets | Less accurate than trees/SVM |
| Handles multi-class naturally | Sensitive to feature scaling |

### SVM (Support Vector Machine)

High-accuracy classifier with RBF kernel:

```
Fit SVM Multi-Class
    │
    ├── Database: (training data)
    │
    └── Model ──▶ (trained SVM ensemble)
```

**When to use:**
- Maximum accuracy needed
- Smaller datasets (< 10,000 samples)
- Complex decision boundaries

**Notes:**
- Uses One-vs-All strategy for multi-class
- Gaussian (RBF) kernel by default
- Slower training than trees/Naive Bayes

## Regression Models

### Linear Regression

Predict continuous values:

```
Fit Linear Regression
    │
    ├── Database: (training data with numeric targets)
    │
    └── Model ──▶ (trained linear model)
```

**When to use:**
- Predicting continuous values
- Understanding feature importance
- Linear relationship expected

**Getting Coefficients:**
```
Get Linear Coefficients
    │
    ├── Model: (trained linear regression)
    │
    └── Info ──▶ {
        coefficients: [0.5, -0.3, 0.8],
        intercept: 2.1,
        n_features: 3
    }
```

## Clustering Models

### K-Means

Partition data into k clusters:

```
Fit KMeans
    │
    ├── Database: (data with records column)
    ├── Clusters: 5  (number of clusters)
    │
    └── Model ──▶ (trained KMeans)
```

**When to use:**
- You know the number of clusters
- Clusters are roughly spherical
- Customer segmentation, grouping

**Getting Centroids:**
```
Get KMeans Centroids
    │
    ├── Model: (trained KMeans)
    │
    └── Info ──▶ {
        k: 5,
        dimensions: 3,
        centroids: [[...], [...], ...]
    }
```

### DBSCAN

Density-based clustering:

```
Fit DBSCAN
    │
    ├── Database: (data with records column)
    ├── Epsilon: 0.5  (max distance between points)
    ├── Min Points: 5  (points to form dense region)
    │
    ├── End ──▶ (clustering complete)
    ├── N Clusters ──▶ (number found)
    └── N Noise ──▶ (outliers found)
```

**When to use:**
- Unknown number of clusters
- Need to detect outliers/anomalies
- Non-spherical cluster shapes

:::note[DBSCAN Returns Counts, Not a Model]
Unlike KMeans, DBSCAN doesn't produce a reusable model—it assigns clusters directly to your data.
:::

## Dimensionality Reduction

### PCA (Principal Component Analysis)

Reduce feature dimensions:

```
Fit PCA
    │
    ├── Database: (high-dimensional data)
    ├── N Components: 2  (target dimensions)
    ├── Output Column: "reduced"
    │
    ├── End ──▶ (reduction complete)
    └── Vectors ──▶ (reduced vectors)
```

**When to use:**
- Too many features (high-dimensional data)
- Preparing for visualization (reduce to 2-3D)
- Removing noise/redundant features

## Making Predictions

The **Predict** node works with any trained model:

### Predict on Database

```
Predict
    │
    ├── Model: (any trained ML model)
    ├── Mode: "Database"
    ├── Database: (data to predict)
    ├── Input Column: "records"
    ├── Output Column: "predictions"
    ├── Batch Size: 5000
    │
    ├── End ──▶ (predictions complete)
    └── Database ──▶ (with predictions column)
```

### Predict on Vector

For single predictions:

```
Predict
    │
    ├── Model: (trained model)
    ├── Mode: "Vector"
    ├── Vector: [1.5, 2.3, 0.8, ...]  (features)
    │
    └── Prediction ──▶ "class_a"  (or numeric value)
```

## Model Evaluation

### Classification Metrics

**Accuracy:**
```
Evaluate Accuracy
    │
    ├── Database: (with predictions & targets)
    ├── Prediction Column: "predictions"
    ├── Target Column: "targets"
    │
    └── Result ──▶ {
        accuracy: 0.92,
        correct: 920,
        total: 1000
    }
```

**Confusion Matrix:**
```
Evaluate Confusion Matrix
    │
    ├── Database: (with predictions & targets)
    ├── Prediction Column: "predictions"
    ├── Target Column: "targets"
    │
    └── Result ──▶ {
        matrix: [[45, 5], [3, 47]],
        precision: [0.94, 0.90],
        recall: [0.90, 0.94],
        f1_score: [0.92, 0.92]
    }
```

### Regression Metrics

```
Evaluate Regression
    │
    ├── Database: (with predictions & targets)
    ├── Prediction Column: "predictions"
    ├── Target Column: "targets"
    │
    └── Result ──▶ {
        mse: 0.05,
        rmse: 0.22,
        mae: 0.18,
        r_squared: 0.89
    }
```

**Metric Guide:**
| Metric | Description | Good Value |
|--------|-------------|------------|
| MSE | Mean Squared Error | Lower is better |
| RMSE | Root MSE (same units as target) | Lower is better |
| MAE | Mean Absolute Error | Lower is better |
| R² | Variance explained | Closer to 1.0 |

## Saving and Loading Models

### Save Model

```
Save ML Model
    │
    ├── Model: (trained model)
    ├── Path: (FlowPath for output)
    │
    └── End
```

Formats:
- **JSON** – Human-readable, portable
- **Binary** – Faster, smaller (Fory format)

### Load Model

```
Load ML Model
    │
    ├── Path: (FlowPath to saved model)
    │
    └── Model ──▶ (loaded model ready for predictions)
```

## ONNX Models (Deep Learning)

For pre-trained neural networks:

### Loading ONNX Models

```
Load ONNX
    │
    ├── Path: (FlowPath to .onnx file)
    │
    └── Session ──▶ (ONNX inference session)
```

### Image Classification (TIMM)

Use models exported from PyTorch Image Models:

```
ONNX Classification
    │
    ├── Session: (ONNX session)
    ├── Image: (image data)
    ├── Top K: 5
    │
    └── Results ──▶ [
        {class_idx: 281, score: 0.92},
        {class_idx: 282, score: 0.05},
        ...
    ]
```

### Object Detection (YOLO/D-FINE)

Detect objects in images:

```
ONNX Detection
    │
    ├── Session: (ONNX session)
    ├── Image: (image data)
    ├── Confidence: 0.5
    ├── NMS Threshold: 0.4
    │
    └── Detections ──▶ [
        {class_idx: 0, score: 0.95, x1: 10, y1: 20, x2: 100, y2: 150},
        ...
    ]
```

### Teachable Machine

For Google Teachable Machine models:

```
Teachable Machine
    │
    ├── Path: (FlowPath to .tflite)
    ├── Labels: (optional labels file)
    ├── Image: (image data)
    │
    └── Results ──▶ [{label: "cat", score: 0.95}, ...]
```

## Model Selection Guide

| Use Case | Recommended Model |
|----------|-------------------|
| Quick classification baseline | Naive Bayes |
| Need to explain predictions | Decision Tree |
| Maximum accuracy (small data) | SVM |
| Predict continuous values | Linear Regression |
| Group data (known K) | K-Means |
| Find outliers & groups | DBSCAN |
| Reduce dimensions | PCA |
| Classify images | ONNX (TIMM) |
| Detect objects | ONNX (YOLO) |
| Imbalanced classes | Use Stratified Split first |

## Complete Example: Customer Churn Prediction

```
┌────────────────────────────────────────────────────────────┐
│                                                            │
│  Load CSV (customer data)                                  │
│       │                                                    │
│       ▼                                                    │
│  Insert to Database                                        │
│       │                                                    │
│       ▼                                                    │
│  Stratified Split (80/20)                                  │
│       │                                                    │
│       ├──▶ Train Set ──▶ Fit Decision Tree                │
│       │                        │                           │
│       │                        ▼                           │
│       │                   Model ────────────┐              │
│       │                                     │              │
│       └──▶ Test Set ────────────────────────┼──▶ Predict  │
│                                             │       │      │
│                                             │       ▼      │
│                                    Confusion Matrix        │
│                                             │              │
│                                             ▼              │
│                                    Save Model (if good)    │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

## Best Practices

### 1. Always Split Your Data
Never evaluate on training data—it gives overly optimistic results.

### 2. Start Simple
Begin with Naive Bayes or Decision Trees, then try more complex models.

### 3. Use Stratified Splitting for Classification
Especially important when classes are imbalanced.

### 4. Check Feature Scaling
Some algorithms (SVM, K-Means) are sensitive to feature scales. Consider normalizing.

### 5. Evaluate Multiple Metrics
Accuracy alone can be misleading. Check precision, recall, and F1.

### 6. Save Good Models
Don't retrain every time—save and load trained models.

## Troubleshooting

### "Model performs poorly"
- Check for data quality issues
- Try a different algorithm
- Increase training data
- Check for class imbalance

### "Training is slow"
- Reduce dataset size with sampling
- Use smaller batch sizes
- Try simpler algorithms (Naive Bayes)

### "Memory errors"
- Set MAX_RECORDS limit
- Process in batches
- Use sampling for very large datasets

## Next Steps

With trained models:

- **[Data Visualization](/topics/datascience/visualization/)** – Visualize predictions and metrics
- **[AI-Powered Analysis](/topics/datascience/ai-analysis/)** – Combine ML with GenAI
- **[Data Loading](/topics/datascience/loading/)** – Work with more data sources
