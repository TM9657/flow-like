---
title: Set History Frequency Penalty
description: Sets the frequency_penalty attribute in a ChatHistory
---

## Purpose of the Node
This node allows you to set the frequency penalty value for a chat history. The frequency penalty controls the likelihood of the model generating previously generated tokens. This can help to control repetition and encourage the model to be more diverse in its responses.

## Pins
| Pin Name         | Pin Description                                     | Pin Type | Value Type |
|------------------|-----------------------------------------------------|----------|------------|
| Start            | Initiate Execution                                  | Execution| Normal     |
| History          | ChatHistory                                         | Struct   | Array      |
| Frequency Penalty| Frequency Penalty Value                             | Float    | Normal     |
| End              | Done with the Execution                             | Execution| Normal     |
| History          | Updated ChatHistory                                 | Struct   | Array      |