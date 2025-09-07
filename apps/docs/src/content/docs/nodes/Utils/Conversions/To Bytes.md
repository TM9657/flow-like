---
title: ToBytes
description: Converts a Struct to Bytes
---

## Purpose of the Node
The ToBytes node converts a given Struct into a byte array. It provides an option to either pretty print the JSON representation of the Struct before converting it to bytes.

## Pins
The node has three pins: an input for the Struct to convert, a boolean to determine if the Struct should be pretty printed, and an output for the resulting byte array.

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| value | The input Struct to be converted to bytes | Struct | VariableType::Generic |
| pretty | A boolean indicating if the Struct should be pretty printed before conversion | Boolean | VariableType::Boolean |
| bytes | The output byte array representing the Struct | Bytes | ValueType::Array |