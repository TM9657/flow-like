---
title: Runtime Variables
description: Configure sensitive values and secrets per-device without storing them in your flows
sidebar:
  order: 25
---

**Runtime Variables** let you configure sensitive data—like API keys, passwords, or personal tokens—**locally on each device**, without embedding them in your flow definitions. This keeps your secrets secure, even when sharing apps or syncing online.

## Quick Start

1. Open your app in Flow-Like Desktop
2. In the sidebar, go to **Settings** → **Runtime Variables**
3. Find the board with variables marked as "runtime configured"
4. Enter your values and click **Save**
5. Run your flow—your configured values will be used automatically

:::tip[New to Variables?]
If you're unfamiliar with variables in Flow-Like, start with the [Variables guide](/studio/variables/) first.
:::

## Why Use Runtime Variables?

Consider this common scenario: You build a flow that connects to an external service using an API key. You have several options:

| Approach | Problem |
|----------|---------|
| Hardcode the key in a node | Exposed if you share the app or commit to version control |
| Use a regular variable with a default value | Still stored in the flow definition |
| **Use a runtime variable** | ✓ Stored locally, never leaves your device |

Runtime variables solve the security problem by keeping sensitive values **separate from your flow definitions**.

### Benefits

- **Security**: Secrets are stored only on your device, encrypted in local storage
- **Multi-device friendly**: Each device can have its own configuration (e.g., dev vs. prod credentials)
- **Safe sharing**: Share apps freely—recipients configure their own values
- **Team collaboration**: Team members use their personal API keys without exposing them to others

## How to Create a Runtime Variable

### 1. Mark a Variable as Runtime Configured

In the Studio, open your board's variables panel and select the variable you want to configure at runtime:

1. Click the variable to open its settings
2. Enable **Runtime Configured**
3. Optionally, also enable **Secret** for additional protection (hides the value when displayed)

Once marked as runtime configured, the variable's default value in the flow definition is ignored. Instead, Flow-Like uses the value you configure in the Runtime Variables settings.

### 2. Configure the Value

After marking a variable as runtime configured:

1. Open your app's **Settings** → **Runtime Variables**
2. Find the board containing your variable
3. Enter the value and click **Save**

The value is now stored locally and will be used whenever that flow runs.

:::note[First-time execution]
If you run a flow with unconfigured runtime variables, Flow-Like will prompt you to enter the missing values before execution.
:::

## Secrets vs. Regular Runtime Variables

You can combine **Runtime Configured** with the **Secret** flag for additional security:

| Setting | Stored Locally | Hidden in UI | Sent to Remote Execution |
|---------|----------------|--------------|--------------------------|
| Runtime Configured only | ✓ | ✗ | ✓ |
| Runtime Configured + Secret | ✓ | ✓ | ✗ |

**Secrets are never sent to remote execution.** If your board is configured for [remote execution](/enterprise/remote-execution/), any variable marked as both runtime configured and secret will only be available when running locally.

## Execution Modes and Runtime Variables

How runtime variables behave depends on your board's [execution mode](/apps/boards/#execution-modes):

### Local Execution
All runtime variables (including secrets) are available. Values are read from your local configuration and passed directly to the executing flow.

### Remote Execution
- Regular runtime variables: Sent to the remote server
- **Secrets: Never sent** (excluded from the payload for security)

### Hybrid Execution
Flow-Like determines at runtime whether to execute locally or remotely based on node requirements. Secrets are only available for the locally-executed portions.

:::caution[Remote Execution Warning]
If a flow running remotely requires a secret variable, it will use the variable's default value (if any) or fail. Design your remote-capable flows to not depend on secrets, or use [execution stages](/enterprise/execution-stages/) to separate sensitive operations.
:::

---

## Technical Details

<details>
<summary>How are values stored?</summary>

Runtime variable values are stored in an encrypted IndexedDB database on your device. Each value is associated with:
- The **app ID**
- The **board ID** where the variable is defined
- The **variable ID**

This means the same variable in different boards (or apps) can have different configured values.

</details>

<details>
<summary>How are values transmitted during execution?</summary>

When you run a flow, runtime variables are included in the execution payload alongside other run information (like the starting node ID). The payload looks conceptually like this:

```json
{
  "id": "starting-node-id",
  "payload": { /* input data */ },
  "runtime_variables": {
    "variable-uuid-1": { "name": "API_KEY", "default_value": [/* encrypted bytes */], ... },
    "variable-uuid-2": { "name": "DB_HOST", "default_value": [/* encrypted bytes */], ... }
  }
}
```

The execution engine merges these values with the board's variable definitions, giving runtime variables priority over any hardcoded defaults.

For remote execution, secrets are filtered out before the payload is sent to the server.

</details>

<details>
<summary>What happens when a runtime variable is missing?</summary>

If you start an execution and a runtime-configured variable hasn't been set:

1. **Automatic prompt**: Flow-Like displays a dialog asking you to enter the missing values
2. **Save and continue**: Once you provide the values, they're saved to local storage and execution proceeds
3. **Cancel**: You can cancel if you don't want to configure them yet

This ensures you never accidentally run a flow with missing configuration.

</details>

<details>
<summary>Can I use runtime variables in events?</summary>

Yes! Events use the same runtime variable system. When an event triggers a flow:

1. Flow-Like checks if the associated board has runtime-configured variables
2. If values are configured, they're included in the execution
3. If values are missing, the event execution is blocked until you configure them (you'll see a notification)

Events always execute locally, so secrets are always available for event-triggered flows.

</details>

---

## Best Practices

### Do's

- ✓ Use runtime variables for API keys, tokens, and passwords
- ✓ Mark truly sensitive values as both **Runtime Configured** and **Secret**
- ✓ Document which variables need configuration (use the variable description field)
- ✓ Use meaningful variable names like `OPENAI_API_KEY` or `DATABASE_PASSWORD`

### Don'ts

- ✗ Don't store secrets in regular variables with default values
- ✗ Don't assume secrets will be available in remote execution
- ✗ Don't share your Flow-Like local storage/database files

## Related Topics

- [Variables in Studio](/studio/variables/) — Learn about all variable types
- [Boards](/apps/boards/) — Understanding boards and execution modes
- [Offline vs. Online](/apps/offline-online/) — How app data is stored and synced
- [Sharing Apps](/apps/share/) — Safely share apps with others
