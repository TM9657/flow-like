---
title: Switch On Bit
description: Routes execution based on the type of the Bit.
---

## Purpose of the Node
The Switch On Bit node evaluates a Bit input and routes the execution to different branches based on the type of the Bit. This allows for conditional execution paths within a visual script.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | Initiate Execution | Execution | Normal |
| Bit | Input Bit | Struct | Bit |
| LLM | Execution if Bit is LLM | Execution | Normal |
| VLM | Execution if Bit is VLM | Execution | Normal |
| Embedding | Execution if Bit is Embedding | Execution | Normal |
| Image Embedding | Execution if Bit is ImageEmbedding | Execution | Normal |
| File | Execution if Bit is File | Execution | Normal |
| Media | Execution if Bit is Media | Execution | Normal |
| Template | Execution if Bit is Template | Execution | Normal |
| Tokenizer | Execution if Bit is Tokenizer | Execution | Normal |
| Tokenizer Config | Execution if Bit is TokenizerConfig | Execution | Normal |
| Special Tokens Map | Execution if Bit is SpecialTokensMap | Execution | Normal |
| Config | Execution if Bit is Config | Execution | Normal |
| Course | Execution if Bit is Course | Execution | Normal |
| Preprocessor Config | Execution if Bit is PreprocessorConfig | Execution | Normal |
| Projection | Execution if Bit is Projection | Execution | Normal |
| Project | Execution if Bit is Project | Execution | Normal |
| Board | Execution if Bit is Board | Execution | Normal |
| Other | Execution if Bit is Other | Execution | Normal |
| Bit Out | Output Bit | Struct | Bit |