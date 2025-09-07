---
title: Set Model Hint
description: A node that sets the model hint in BitModelPreference.
---

## Purpose of the Node
The Set Model Hint node is used to modify the model hint within a BitModelPreference structure. It accepts the current preferences and a new model hint, then outputs the updated preferences.

## Pins

| Pin Name        | Pin Description                | Pin Type | Value Type |
|-----------------|--------------------------------|----------|------------|
| Start           | Initiate Execution              | Execution| N/A        |
| Preferences     | Current Preferences             | Struct   | BitModelPreference |
| Model Hint      | Model Hint to set               | String   | N/A        |
| End             | Done with the Execution         | Execution| N/A        |
| Preferences     | Updated Preferences             | Struct   | BitModelPreference |