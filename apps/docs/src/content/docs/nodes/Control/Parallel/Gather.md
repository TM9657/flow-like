---
title: Gather
description: A Gather
---

## Purpose of the Node
The Gather node is designed to wait for all input execution states to be in sync before proceeding. It takes multiple execution input pins and outputs an execution done pin, which activates once all input states have synchronized.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Input Pin | Execution | Boolean |
| Start | Input Pin | Execution | Boolean |
| exec_done | In Sync | Execution | Boolean |