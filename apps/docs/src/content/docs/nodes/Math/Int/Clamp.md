---
title: Clamp Integer
description: Clamps an integer within a specified range.
---

## Purpose of the Node
The Clamp Integer node clamps an integer value within a specified minimum and maximum range. It is useful for ensuring that a value stays within a certain range, preventing it from going below a minimum or exceeding a maximum.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Integer | The integer value to be clamped. | Input | Integer |
| Min | The minimum value that the integer can be clamped to. | Input | Integer |
| Max | The maximum value that the integer can be clamped to. | Input | Integer |
| Clamped | The clamped value, which will be within the range specified by Min and Max. | Output | Integer |