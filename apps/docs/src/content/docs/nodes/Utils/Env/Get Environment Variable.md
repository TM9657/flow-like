---
title: Get Environment Variable
description: Retrieves the value of an environment variable specified by its key.
---

## Purpose of the Node
The Get Environment Variable node retrieves the value of an environment variable based on the key provided. It outputs the value of the environment variable and a boolean indicating whether the variable was found.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Key** | The key of the environment variable to retrieve. | String | Map |
| **Variable** | The value of the environment variable. | String | Map |
| **Found?** | Boolean indicating whether the environment variable was found. | Boolean | Set |