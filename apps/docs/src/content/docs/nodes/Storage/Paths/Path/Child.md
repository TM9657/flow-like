---
title: Create Child Path
description: Creates a child path from a parent path
---

## Purpose of the Node
The Create Child Path node is designed to generate a child path from a specified parent path and a given child name. This is useful for navigating or constructing paths in a hierarchical data structure.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Trigger to initiate the node execution | None | None |
| parent_path | The parent FlowPath from which the child path will be created | Struct | Array |
| child_name | The name of the child path to be created | String | Map |
| path | The resulting child path | Struct | Array |
| End | Trigger indicating the end of the node execution | None | None |