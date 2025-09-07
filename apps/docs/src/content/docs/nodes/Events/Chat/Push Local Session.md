---
title: Push Local Session
description: A node that pushes a new local session to the chat. The session persists for one chat session.
---

## Purpose of the Node
This node is used to push a new local session to the chat. The session persists for one chat session and can be used to store temporary data.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Input, Initiates Execution | Execution | N/A |
| local_session | Local Session, Generic Struct Type | Struct | Array |
| End | Output, Indicates completion of Execution | Execution | N/A |