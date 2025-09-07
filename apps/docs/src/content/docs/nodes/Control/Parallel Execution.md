---
title: Parallel Execution Node
description: A node that triggers multiple sub-nodes in parallel.
---

## Purpose of the Node
The Parallel Execution Node is designed to execute multiple connected nodes in parallel, either using tasks or threads, based on the configuration. This allows for efficient handling of multiple workflows at once.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Start** | Trigger pin to start the parallel execution. | Execution | - |
| **Threads** | Configures whether to use tasks or threads for parallel execution. Valid values are "tasks" and "threads". | String | - |
| **Done** | Output pin indicating that all parallel executions are complete. | Execution | - |
| **Output** | Outputs the results of the parallel executions. | Execution | - |
| **Output** | Outputs the results of the parallel executions. | Execution | - |