---
title: Format String
description: Formats a string with placeholders using dynamic inputs.
---

## Purpose of the Node
The Format String node takes a template string containing placeholders and replaces them with the actual values provided as inputs. The placeholders are specified in the format `{placeholder_name}` within the string. Each placeholder is dynamically generated based on the values present in the input string.

## Pins
| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| format_string | Input string containing placeholders | String | Map |
| placeholder_name | Dynamic input pin for each placeholder found in the format_string | String | Normal |
| formatted_string | Formatted output string | String | Map |

The `format_string` pin is the main input where you provide the string containing placeholders. The node will automatically generate input pins for each placeholder found. When you provide values for these placeholder pins, they will replace the placeholders in the `format_string` to produce the `formatted_string` output.