---
title: Set Weight
description: Adjusts the weight of a specified preference in the model preferences.
---

## Purpose of the Node
The Set Weight node allows you to adjust the weight of a specific preference in the model preferences. You can select a preference from the available options and set a weight between 0.0 and 1.0. The updated preferences are then outputted.

## Pins
| Pin Name        | Pin Description                                              | Pin Type | Value Type |
|:----------------:|:------------------------------------------------------------:|:--------:|:--------:|
| **Start**         | Initiate Execution                                           | Execution |          |
| **Preferences In** | Current preferences in BitModelPreference format             | Struct   |          |
| **Preferences Key** | Key of the preference to adjust (e.g., Cost, Speed)           | String   |          |
| **Weight**        | Weight value between 0.0 and 1.0 to set for the specified preference | Float |          |
| **End**           | Output indicating that execution is complete                     | Execution |          |
| **Preferences Out** | Updated preferences with the specified preference weight adjusted | Struct   |          |