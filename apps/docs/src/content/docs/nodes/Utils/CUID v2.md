---
title: CUID Node
description: Generates a Collision Resistant Unique Identifier
---

## Purpose of the Node
The CUID Node is used to generate a CUID (Collaborative Unique IDentifier) which is a string-based identifier designed to be unique across space and time. It is commonly used in distributed systems to generate unique IDs for documents, keys, etc.

## Pins
The CUID Node has two output pins: one for the execution signal and one for the generated CUID.

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Trigger to start the CUID generation process | Execution | N/A |
| End | Output pin indicating the completion of the CUID generation | Execution | N/A |
| Cuid | The generated CUID | String | N/A |