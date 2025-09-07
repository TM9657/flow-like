---
title: Attachment From URL
description: Retrieves an attachment from a signed URL.
---

## Purpose of the Node
The Attachment From URL node is designed to take a signed URL as input and return an attachment. This node is particularly useful when dealing with file attachments in chat or document management systems where direct URLs to the files are provided.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Signed URL | The signed URL from which the attachment will be retrieved | String | Map |
| Attachment | The attachment retrieved from the signed URL | Struct | Array |