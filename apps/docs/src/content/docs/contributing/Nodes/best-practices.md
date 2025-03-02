---
title: Best Practices
description: Creating a Node should be a straight forwards and easy task for most.
sidebar:
  order: 1
---

Some of the best practices we recommend for the creation of great nodes.

## Contribute vs Local Node
Currently we only offer the creation of nodes via a pull request into the project (or your fork). In the future we will add the option to create nodes in either Lua or WebAssembly.

## Best Practices
Some of the best practices

#### 1. Early Returns
Keep your nodes readable for our code audits. Part of this is follow standard coding best practices as much as possible.

#### 2. Prefer Execution Path Failures on Execution Nodes
You should catch and handle errors gracefully. You have two main options for this.
1. Add a Failed Execution Pin. This will help the user to write custom logic, catching the error on their side.
2. For Pure Nodes (the ones without Execution Pins), you can use a success Boolean Pin.

If you have the option to use the Failed Execution Pin, we prefer that.

```rust title="Failed Execution Path"
// From the Pop Array Node
 async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        // We initialize by setting the failed path. This node might be executed multiple times.
        // Resetting its state is good practice and makes sure we don't get strange behavior.
        context.activate_exec_pin("failed").await?;
        context.deactivate_exec_pin("exec_out").await?;
        let array_in: Vec<Value> = context.evaluate_pin("array_in").await?;
        let mut array_out = array_in.clone();
        let popped_value = array_out.pop();
        let success = popped_value.is_some();

        context.set_pin_value("array_out", json!(array_out)).await?;
        if let Some(value) = popped_value {
            context.set_pin_value("value", json!(value)).await?;
        }

        // In case of success, we activate the normal execution path, deactivating the failed route.
        if success {
            context.deactivate_exec_pin("failed").await?;
            context.activate_exec_pin("exec_out").await?;
        }

        Ok(())
    }
```

#### 3. Use Logging
Show your users what went wrong. Do not rely on the Runtime to Log the error that made your Node Fail. Handle the error and return more meaningful errors. You can also use this to warn the user, just print information that might be helpful or Debugging Information.

```rust title="Logging is easy,"
context.log_message(
    "Your Error",
    crate::flow::execution::LogLevel::Error,
);
```