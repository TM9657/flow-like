---
title: Push Global Session
description: Pushes a new global session to the chat, persisting for all chat sessions.
---

## Purpose of the Node
The Push Global Session node is designed to push a new global session to the chat. This session persists for all chat sessions, allowing for consistent global state management.

## Pins
The Push Global Session node has the following pins:

| Pin Name   | Pin Description                   | Pin Type | Value Type |
|:----------:|:---------------------------------:|:------:|:------:|
| Start      | Initiate Execution                | Struct | Execution |
| global_session | Generic Struct Type              | Struct | Map |
| End        | Done with the Execution             | Struct | Execution |