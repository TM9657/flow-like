---
title: Set History User
description: Updates the user attribute in a ChatHistory
---

## Purpose of the Node
The Set History User node updates the user attribute in a ChatHistory. It takes the input ChatHistory and a user value, then outputs the updated ChatHistory.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Exec | Execution |
| history | ChatHistory | Struct | History |
| user | User Value | String | String |
| End | Done with the Execution | Exec | Execution |
| history_out | Updated ChatHistory | Struct | History |