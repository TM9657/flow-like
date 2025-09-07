---
title: Sørensen-Dice Coefficient
description: Calculates the Sørensen-Dice coefficient between two input strings.
---

## Purpose of the Node
The Sørensen-Dice Coefficient Node calculates the similarity between two input strings using the Sørensen-Dice coefficient, which is a statistic used to gauge the similarity between two sample sets.

## Pins
The node has two input pins and one output pin.

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | The node starts processing when this pin is triggered. | Start | N/A |
| End | The node ends processing and outputs the result when this pin is triggered. | End | N/A |
| String 1 | The first input string for the Sørensen-Dice coefficient calculation. | Input | String |
| String 2 | The second input string for the Sørensen-Dice coefficient calculation. | Input | String |
| Coefficient | The calculated Sørensen-Dice coefficient between the two input strings. | Output | Float |