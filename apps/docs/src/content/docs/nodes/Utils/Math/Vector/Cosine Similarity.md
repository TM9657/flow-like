---
title: Float Vector Cosine Similarity
description: Calculates the cosine similarity of two float vectors
---

## Purpose of the Node
This node calculates the cosine similarity between two float vectors. It is useful for measuring the cosine of the angle between two non-zero vectors in a multi-dimensional space. The cosine similarity ranges from -1 to 1, where 1 indicates vectors that are identical, 0 indicates orthogonal vectors, and -1 indicates vectors that are diametrically opposed.

## Pins
- **Vector 1**: The first float vector.
- **Vector 2**: The second float vector.
- **Similarity**: The cosine similarity of the two vectors.

| Pin Name       | Pin Description                                                                                   | Pin Type | Value Type |
|----------------|---------------------------------------------------------------------------------------------------|----------|------------|
| Vector 1       | The first float vector.                                                                            | Struct   | Array      |
| Vector 2       | The second float vector.                                                                           | Struct   | Array      |
| Similarity     | The cosine similarity of the two vectors.                                                            | Struct   | Float      |