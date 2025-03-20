---
title: Variable & Value Types
description: Understanding the data types and value structures in Flow Like
sidebar:
  order: 0
---

Understanding the different variable and value types is essential for creating efficient and effective flows. Flow Like provides a comprehensive type system that balances simplicity with the power needed for complex operations.

## Data Types

Flow Like supports various data types that map to their Rust equivalents under the hood:

| Type     | Rust Type       | Category  | Description | Example |
|----------|-----------------|-----------|-------------|---------|
| Boolean  | bool            | Primitive | Represents true or false values | `true`, `false` |
| Integer  | i64             | Primitive | Whole numbers without decimal points | `42`, `-7`, `0` |
| Float    | f64             | Primitive | Numbers with decimal precision | `3.14`, `-0.01`, `2.0` |
| Byte     | u8              | Primitive | Single byte values (0-255) | `65`, `0x41` |
| String   | String          | Complex   | Text data of any length | `"Hello, Flow Like!"` |
| DateTime | DateTime<Utc>   | Complex   | Date and time values with timezone information | `2023-10-15T08:30:00Z` |
| PathBuf  | PathBuf         | Complex   | File system paths with platform-specific handling | `"/home/user/documents"` |
| Struct   | Value           | Complex   | Custom structured data composed of multiple fields | `{ "name": "Task", "complete": false }` |

### Working with Primitive Types

Primitive types are the building blocks for more complex data structures. They are stored by value and have fixed memory requirements:

- **Boolean**: Used for conditional operations, flow control, and logical comparisons
- **Integer**: Ideal for counting, indexing, and whole number arithmetic
- **Float**: Appropriate for scientific calculations, percentages, and any value requiring decimal precision
- **Byte**: Efficient for dealing with binary data, file operations, and ASCII character representation

### Working with Complex Types

Complex types manage memory differently and can represent more sophisticated data:

- **String**: Used for text processing, user inputs, and data that needs to be human-readable
- **DateTime**: Essential for scheduling, logging, time-based operations, and managing temporal data
- **PathBuf**: Used for file system operations, maintaining compatibility across different operating systems
- **Struct**: Creates custom data structures for domain-specific requirements

## Value Types

Flow Like supports several value type structures that help organize and manipulate data in different ways:

| Value Type    | Description | Common Use Cases | Example |
|---------------|-------------|------------------|---------|
| Normal Value  | A single value of any data type | Simple variables, return values | `42`, `"text"`, `true` |
| Arrays        | Ordered collections of values with index-based access | Lists, sequences, batched processing | `[1, 2, 3, 4]`, `["red", "green", "blue"]` |
| HashSet       | Unordered collections of unique values | Deduplication, membership testing, unions/intersections | `{1, 2, 3}` (no duplicates allowed) |
| HashMap       | Key-value pairs with unique keys | Lookups, configuration settings, attribute storage | `{"name": "Flow", "version": "1.0"}` |

### When to Use Different Value Types

- **Normal Values**: Use when you only need to store a single piece of information
- **Arrays**: Use when order matters and you need to process items sequentially
- **HashSet**: Use when you need to ensure uniqueness or perform set operations
- **HashMap**: Use when you need to associate values with specific keys for quick retrieval

## Type Conversion

You can convert between certain types as needed in your flows:
- Integers to Floats: `42 → 42.0`
- Strings to other types: `"42" → 42`, `"true" → true`
- Arrays to HashSets: `[1, 1, 2] → {1, 2}`

## Best Practices

- Choose the simplest type that meets your requirements
- Use HashSets when uniqueness is important
- Prefer strongly typed structures for complex data
- Consider performance implications when working with large collections
- Document custom struct schemas for better maintainability

Each type and structure in Flow Like has been carefully selected to provide the flexibility needed for automation while maintaining strong type safety.