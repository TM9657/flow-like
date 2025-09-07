---
title: Attachment to URL
description: Converts an attachment to a signed URL.
---

## Purpose of the Node
The Attachment to URL node processes an attachment and returns a signed URL. It checks if the attachment is an URL and sets the "Success" pin to true if it is. Otherwise, it sets the "Success" pin to false.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| **Attachment** | The attachment to convert to a signed URL. | Struct | Attachment |
| **Signed URL** | The signed URL of the attachment. | String | String |
| **Success** | Indicates if the attachment was successfully converted to a URL. | Boolean | Boolean |