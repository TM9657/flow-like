---
title: Hash File
description: A node that hashes a file given a path.
---

## Purpose of the Node
This node takes a file path, hashes the file using the associated storage, and outputs the resulting hash. It also provides a failure output pin in case the hashing process fails.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Exec | - |
| Path | FlowPath to the file to be hashed | Struct | FlowPath |
| End | Done with the Execution | Exec | - |
| Hash | The resulting hash of the file | String | - |
| Failed | Outputs if the file hashing fails | Exec | - |