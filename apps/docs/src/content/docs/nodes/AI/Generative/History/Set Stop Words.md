---
title: Set History Stop Words
description: A node that sets the stop_words attribute in a ChatHistory.
---

## Purpose of the Node
This node allows you to set the stop_words attribute in a ChatHistory. It takes in a ChatHistory and a list of stop words as input, and outputs the updated ChatHistory.

## Pins

| Pin Name       | Pin Description                   | Pin Type | Value Type |
|----------------|-----------------------------------|----------|------------|
| Start          | Initiate Execution                | Execution| None       |
| History        | ChatHistory                       | Struct   | History    |
| Stop Words     | Stop Words Value                  | String   | Array      |
| End            | Done with the Execution           | Execution| None       |
| History        | Updated ChatHistory               | Struct   | History    |