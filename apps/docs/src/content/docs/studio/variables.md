---
title: Variables
description: Typed Variables
sidebar:
  order: 40
---

Variables act as shared, in-memory storage at the board level. Typical scenarios for using variables include:
- Configuring the name of a database and reusing it across multiple flows (e.g., for ingestion and retrieval).
- Recursively scraping a website for outgoing links and pushing each link into an array.
- During each *While Loop* iteration, evaluating whether a certain condition is still *true*. The condition is set somewhere downstream in the *While Loop* leaf branch.

All flows within a board can read and write variables through the *Get Variable* and *Set Variable* nodes:
- To *read* a variable, either search the node catalog for *Get variable_name* or drag and drop the variable from the variables menu onto the canvas.
- To *write* a variable, either search for *Set variable_name* or drag it from the variables menu.

![A screenshot showing how to manage variables in Flow-Like Desktop and integrate them in flows](../../../assets/WorkingWithVariables.webp)

To specify a variable *type*, open the variables menu, click the variable you want to configure, and select the type from the drop-down list:
![A screenshot showing how to set the type of a variable](../../../assets/SetVariableType.webp)

To specify a variable *value* (*Single*, *Array*, *Set*, or *Map*), click the pill-shaped color indicator next to *Variable Types*:
![A screenshot showing how to set the value of a variable](../../../assets/SetVariableValue.webp)

## Variable Settings

Each variable has additional settings you can configure:

### Editable
When enabled, the variable can be modified by *Set Variable* nodes during execution.

### Exposed
When enabled, the variable becomes an input parameter when the board is used as a layer or triggered by an event.

### Secret
When enabled, the variable's value is masked in the UI. Useful for sensitive data like API keys. Combine with **Runtime Configured** for maximum security.

### Runtime Configured
When enabled, the variable's value is **not stored in the flow definition**. Instead, you configure it separately in your app's [Runtime Variables settings](/apps/runtime-variables/). This is the recommended approach for:
- API keys and tokens
- Passwords and secrets
- Device-specific configuration
- Values that differ between team members

:::tip[Keeping Secrets Safe]
For sensitive data, enable both **Secret** and **Runtime Configured**. This ensures the value is stored locally on your device, never synced to the cloud, and never sent during remote execution. Learn more in the [Runtime Variables guide](/apps/runtime-variables/).
:::
