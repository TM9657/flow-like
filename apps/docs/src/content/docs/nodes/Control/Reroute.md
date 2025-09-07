---
title: Reroute
description: A Reroute Node
---

## Purpose of the Node
The Reroute Node is used to pass data from an input pin directly to an output pin without any modification. It is useful for control flow scenarios where the data needs to be re-routed through different paths without altering its content.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Start** | The input pin where data is received. | Struct | Array |
| **End** | The output pin where data is sent. | String | Map |