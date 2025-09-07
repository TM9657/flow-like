---
title: Set History Seed
description: Sets the seed attribute in a ChatHistory
---

## Purpose of the Node
The Set History Seed node updates the seed attribute in a provided ChatHistory. This is useful for ensuring reproducibility in generative AI workflows where the seed value can influence the output.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Execution | N/A |
| History | ChatHistory | Struct | History |
| Seed | Seed Value | Integer | N/A |
| End | Done with the Execution | Execution | N/A |
| History | Updated ChatHistory | Struct | History |