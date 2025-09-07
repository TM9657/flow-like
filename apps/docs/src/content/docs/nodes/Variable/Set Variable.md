---
title: Set Variable
description: Sets a variable with a specified name and value.
---

## Purpose of the Node
The Set Variable node is used to set the value of a variable identified by a reference string. It takes in the variable reference and the value to be set as inputs, and outputs a trigger once the variable value is set, as well as the newly set value.

## Pins

| Pin Name    | Pin Description                             | Pin Type | Value Type |
|:-------------:|:-------------------------------------------:|:--------:|:----------:|
| **Start**     | Trigger pin to start the node execution     | Execution| Normal     |
| **Variable Reference** | String representing the reference to the variable | String | String     |
| **Value**      | The new value to be assigned to the variable  | Generic  | Normal     |
| **End**       | Trigger pin once the variable value is set | Execution| Normal     |
| **New Value**  | The newly set value of the variable         | Generic  | Normal     |