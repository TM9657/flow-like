---
title: Push Content
description: A node that pushes content into a HistoryMessage based on the specified type (Text or Image).
---

## Purpose of the Node
This node allows users to push content (either text or image) into a `HistoryMessage` node. Based on the type of content specified, it either appends text or an image to the message and then outputs the modified message. This node is particularly useful in generative AI workflows where message content needs to be dynamically updated.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Triggers the execution of the node. | Execution | Normal |
| Message | Input message of type `HistoryMessage` to which content needs to be appended. | Struct | HistoryMessage |
| Type | The type of content to be pushed into the message. It can either be "Text" or "Image". | String | Normal |
| Text | Input text content to be appended to the message when the type is "Text". This pin is shown only when "Text" is selected as the content type. | String | Normal |
| Image | Input image content to be appended to the message when the type is "Image". This pin is shown only when "Image" is selected as the content type. | String | Normal |
| Mime | Input MIME type of the image content to be appended to the message when the type is "Image". This pin is shown only when "Image" is selected as the content type. | String | Normal |
| End | Indicates the completion of the node's execution. | Execution | Normal |
| Message | The output message with the appended content. | Struct | HistoryMessage |