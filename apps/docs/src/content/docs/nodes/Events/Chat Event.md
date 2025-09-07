---
title: Chat Event
description: A node that processes and emits chat-related events, including history, local and global sessions, actions, and user information.
---

## Purpose of the Node
The Chat Event node is designed to handle and emit chat-related events. It processes incoming chat payloads, extracts relevant information such as chat history, local and global sessions, actions, and user details, and outputs these details via various pins. The node also hooks a completion event that listens for cached chat responses and emits them through an event trigger.

## Pins
The Chat Event node has several output pins:

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Trigger for the start of the event processing. | Exec | - |
| history | Outputs the chat history. | Struct | History |
| local_session | Outputs the local session associated with the chat. | Struct | Struct |
| global_session | Outputs the global session associated with the chat. | Struct | Struct |
| actions | Outputs the actions triggered by the chat. | Struct | Array |
| attachments | Outputs the attachments associated with the chat. | Struct | Array |
| user | Outputs the user information. | Struct | Struct |