---
title: Push Attachment
description: A node that pushes an attachment to the chat.
---

## Purpose of the Node
The Push Attachment node is used to add an attachment to the chat response. It takes in an execution trigger and an attachment object, then pushes the attachment to the chat and outputs an execution completion.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiates the execution of the node. | Execution | None |
| Attachment | The attachment to be pushed to the chat, of type Struct. | Struct | Attachment |
| End | Indicates the completion of the node's execution. | Execution | None |