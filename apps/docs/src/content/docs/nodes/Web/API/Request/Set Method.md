---
title: Set Method
description: Sets the method of an HTTP request
---

## Purpose of the Node
The Set Method node allows you to set the HTTP method (e.g., GET, POST, PUT, DELETE, PATCH) for an HTTP request. This is useful when you need to dynamically change the request method based on certain conditions or inputs.

## Pins

| Pin Name    | Pin Description         | Pin Type | Value Type |
|:------------|:----------------------|:---------|:---------|
| Request In  | The incoming HTTP request | Struct   | HttpRequest |
| Method In   | The method to set for the HTTP request | String   | GET, POST, PUT, DELETE, PATCH |
| Request Out | The updated HTTP request | Struct   | HttpRequest |