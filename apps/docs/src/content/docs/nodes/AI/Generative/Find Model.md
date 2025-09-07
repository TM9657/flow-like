---
title: Find LLM Node
description: A node that finds the best model based on certain selection criteria.
---

## Purpose of the Node
This node is designed to find the best model based on specified preferences. It takes in execution and preferences, processes them, and outputs the selected model and completion execution.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Trigger pin to initiate the model finding process | Execution | - |
| Preferences | Input pin for the preferences for the model, which is a Struct | Struct | - |
| Model | Output pin for the selected model, which is a Struct | Struct | - |
| End | Output pin to indicate the completion of the model finding process | Execution | - |