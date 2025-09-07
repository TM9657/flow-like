---
title: Read to String
description: A node that reads the content of a file to a string.
---

## Purpose of the Node
The **Read to String** node reads the content of a specified file and outputs it as a string. This node is useful for processing text files or extracting data stored in a file into a string format for further manipulation or analysis.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiates the execution of the node. | Execution | N/A |
| Path | The flow path specifying the location of the file to read. | Struct | FlowPath |
| Content | Outputs the content of the file as a string. | String | N/A |
| Failed | Triggers if the file reading process fails. | Execution | N/A |
| End | Indicates that the execution has completed successfully. | Execution | N/A |

**Notes:**
- The **Path** pin uses the `FlowPath` struct, which defines the file path for the node to read from.
- The **Content** pin outputs the content of the file as a string.
- The **Failed** pin triggers if any errors occur during the file reading process.
- The **Start** and **End** pins represent the start and end of the node's execution, respectively.