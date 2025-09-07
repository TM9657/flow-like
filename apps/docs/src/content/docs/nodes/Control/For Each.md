---
title: Loop (For Each)
description: Loops over an Array and processes each element
---

## Purpose of the Node
The Loop node (For Each) is designed to iterate over each item in an array. It triggers an execution pin for each element and passes the current item and its index as output. Once all items have been processed, it triggers a "Done" execution pin.

## Pins
This node has the following pins:

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Start** | Trigger the loop to start processing the array. | Execution | N/A |
| **Array** | The array to loop over. | Generic | Array |
| **For Each Element** | Executes for each element in the array. | Execution | N/A |
| **Value** | The current element value being processed. | Generic | N/A |
| **Index** | The current array index of the element. | Integer | Number |
| **Done** | Executes once the array is completely processed. | Execution | N/A |

The **Start** pin is the entry point to trigger the loop, and **Done** is the exit point after the entire array has been processed. The **For Each Element** pin is triggered for each iteration, allowing downstream nodes to process the current element and its index.