---
title: Hybrid Search Local Database
description: Searches the Database based on a Vector and Text
---

## Purpose of the Node
The Hybrid Search Local Database node is designed to perform a hybrid search operation using both a vector and a full-text search term on a local database. It retrieves items from the database based on the specified criteria and can optionally rerank the results.

## Pins

| Pin Name | Pin Description | Pin Type | Value Type |
|:----------:|:-------------:|:------:|:------:|
| Start | The start of the node execution. | Execution | N/A |
| Database | Database Connection Reference. | Struct | NodeDBConnection |
| Search Term | Full Text Search Term. | String | Normal |
| Vector | Vector to Search. | Float | Array |
| SQL Filter | Optional SQL Filter. | String | Normal |
| Re-Rank | Should the items be reranked? | Boolean | Normal |
| Limit | Limit | Integer | Normal |
| Offset | Offset | Integer | Normal |
| End | The end of the node execution. | Execution | N/A |
| Values | Found Items | Struct | Array |