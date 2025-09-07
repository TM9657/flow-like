---
title: Read Image (URL)
description: Reads an image from a URL and outputs it as a NodeImage object.
---

## Purpose of the Node
Reads an image from a given URL, optionally applies EXIF orientation, and outputs the image as a NodeImage object.

## Pins
| Pin Name     | Pin Description                      | Pin Type | Value Type |
|--------------|--------------------------------------|----------|------------|
| Start        | Initiate Execution                   | Execution|            |
| Signed Url   | The URL of the image to be read      | String   |            |
| Apply Exif   | Whether to apply EXIF orientation    | Boolean  | false      |
| End          | Done with the Execution                | Execution|            |
| Image        | The read image object                  | Struct   | NodeImage  |