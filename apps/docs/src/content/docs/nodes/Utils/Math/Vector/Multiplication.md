---
title: Vector Multiplication
description: Multiplies two float vectors element-wise
---

## Purpose of the Node
The Vector Multiplication node takes two float vectors as input and outputs their element-wise product. This node is useful for performing mathematical operations on vectors where each element of the resulting vector is the product of corresponding elements from the input vectors.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Trigger signal to start the multiplication process. | Exec | None |
| vector1 | The first float vector. | Float | Array |
| vector2 | The second float vector. | Float | Array |
| End | Signal indicating the multiplication has completed. | Exec | None |
| result_vector | The resulting vector containing the element-wise products. | Float | Array |