---
title: Creating Rust Nodes
description: A small guide on how to create a Rust based Node.
---

Nodes follow a very simple creation process. Let us create an example node.

## The Node Object
The Node object is our Logic Wrapper.

### The Node Skeleton

A Node has to implement the `NodeLogic` Trait. Most importantly it has to implement `get_node()` and `run()`.

In the following we have a look at the "**Branch**" Node for Boolean Control Flow, comparable to the IF statement. The Node has to be constructed with a unique name and a friendly name. The unique name is used to determine the logic to execute, so it should not be changed in any way later on.

A Node always has Pins attached to it. There are multiple Pins you can use.

```rust title="Branch Node"
#[async_trait]
impl NodeLogic for BranchNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "control_branch",
            "Branch",
            "Branches the flow based on a condition",
            "Control",
        );
        node.add_icon("/flow/icons/split.svg");

        node.add_input_pin("exec_in", "Input", "Trigger Pin", VariableType::Execution);
        node.add_input_pin(
            "condition",
            "Condition",
            "The condition to evaluate",
            VariableType::Boolean,
        )
        .set_default_value(Some(serde_json::json!(true)));

        node.add_output_pin(
            "true",
            "True",
            "The flow to follow if the condition is true",
            VariableType::Execution,
        );
        node.add_output_pin(
            "false",
            "False",
            "The flow to follow if the condition is false",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let condition = context.evaluate_pin::<bool>("condition").await?;

        let true_pin = context.get_pin_by_name("true").await?;
        let false_pin = context.get_pin_by_name("false").await?;

        if condition {
            context.activate_exec_pin_ref(&true_pin).await?;
            context.deactivate_exec_pin_ref(&false_pin).await?;

            return Ok(());
        }

        context.deactivate_exec_pin_ref(&true_pin).await?;
        context.activate_exec_pin_ref(&false_pin).await?;

        return Ok(());
    }
}

```

### Pure Nodes
Pure Nodes are nodes without Execution Pins. These nodes might be cached by the runtime, so you should only use this type, if you do not produce any sideeffects.
The `Branch` Node from [the previous section](##the-node-skeleton) is an example of a Pure Node.

```rust title="A Pure Node 'Add'"
#[async_trait]
impl NodeLogic for AddIntegerNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new("int_add", "+", "Adds two Integers", "Math/Int");
        node.add_icon("/flow/icons/sigma.svg");

        node.add_input_pin(
            "integer1",
            "Integer 1",
            "Input Integer",
            VariableType::Integer,
        );
        node.add_input_pin(
            "integer2",
            "Integer 2",
            "Input Integer",
            VariableType::Integer,
        );

        node.add_output_pin(
            "sum",
            "Sum",
            "Sum of the two integers",
            VariableType::Integer,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let integer1: i64 = context.evaluate_pin("integer1").await?;
        let integer2: i64 = context.evaluate_pin("integer2").await?;
        let sum = integer1 + integer2;
        context.set_pin_value("sum", json!(sum)).await?;
        Ok(())
    }
}
```

### Dynamic Nodes
You can dynamically update your Node, if you want to. This has many use cases. Some examples are:
1. You have updated your node and you need to update the Pins of it gracefully.
2. Your Node should behave differently, depending on the rest of the board.
3. Your Node should behave differently, depending on the Input of the Node, for example in the `Format` Node bellow, where we add new Pins, based on the Input String and adjust the types of these new Pins depending on the connected type.
4. You are working with Generic Typed Nodes

Most of these use cases are quite advanced. You can find examples for this in our code base, if you look for the `on_update` function.

```rust title="On Update Function for the Format Node"
async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let pins: Vec<_> = node
            .pins
            .values()
            .filter(|p| p.name != "format_string" && p.pin_type == PinType::Input)
            .collect();

        let format_string: String = node
            .get_pin_by_name("format_string")
            .and_then(|pin| pin.default_value.clone())
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
            .and_then(|json| json.as_str().map(ToOwned::to_owned))
            .unwrap_or_default();

        let mut current_placeholders = pins
            .iter()
            .map(|p| (p.name.clone(), *p))
            .collect::<HashMap<_, _>>();

        let mut all_placeholders = HashSet::new();
        let mut missing_placeholders = HashSet::new();

        for cap in self.regex.captures_iter(&format_string) {
            if let Some(placeholder) = cap.get(1).map(|m| m.as_str().to_string()) {
                all_placeholders.insert(placeholder.clone());
                if current_placeholders.remove(&placeholder).is_none() {
                    missing_placeholders.insert(placeholder);
                }
            }
        }

        let ids_to_remove = current_placeholders
            .values()
            .map(|p| p.id.clone())
            .collect::<Vec<_>>();
        ids_to_remove.iter().for_each(|id| {
            node.pins.remove(id);
        });

        for placeholder in missing_placeholders {
            node.add_input_pin(&placeholder, &placeholder, "", VariableType::Generic);
        }

        all_placeholders.iter().for_each(|placeholder| {
            let _ = node.match_type(&placeholder, board.clone(), None);
        })
    }
```
The `on_update` function gets Read Access to the whole board, in case you need to fetch information about it.

:::caution
 Keep in mind, that this function is called on every update to the board, keep it efficient.
:::
:::tip
You can use the `on_update` function to validate the Node and return Error codes that are attached to your node to notify the user that the Node needs some more configuration. You can even dynamically change the name of the Node or Pins or add Comments.
:::

## The Context Object
By now you have probably wondered about the input elements you get for your node. First you get a reference to the state of the app, where you can interact with Model Providers and get a cached HTTP Client.

More important however is the `Context` Object. This context let´s you do multiple things. 

### Pin Interactions
The context object is your best friend interacting with Pins. Most of the time you will use it to read and write to Pins. You can either do so by reference or by name. 
If you have to write to a pin multiple times, the reference one is cheaper.

The Type we use for Communication is the abstract `serde_json:Value`. It allows to write a lot of 

```rust title="Reading and Writing Pins"
// The Type is necessary in this case to guide the evaluate_pin function. This evaluation is by name.
let string: String = context.evaluate_pin("string").await?;

// Setting the "length" Pin with a JSON Value
context.set_pin_value("length", json!(length)).await?;
```

## Pins
Pins are a subcomponent for your Node. They act an interface to other Nodes.

### Pin Options
You can guide the user on how to correctly use your node by setting Options to the Node Pins. These options can be ranges for numbers, Valid Values for Enum like String Pins or Schema enforced Struct Pins.

### Pin Schemas
Struct Pins can have a Schema attached to it. This can help to make sure users are not accidentally connecting invalid Pins. This Schema can be enforced or just a guidance.

```rust title="Setting a Schema"
node.add_input_pin(
            "bit",
            "Model Bit",
            "The Bit that contains the Model",
            VariableType::Struct,
        )
    .set_schema::<Bit>();
```

```rust title="Enforcing a Schema"
node.add_input_pin(
            "bit",
            "Model Bit",
            "The Bit that contains the Model",
            VariableType::Struct,
        )
    .set_schema::<Bit>()
    .set_options(PinOptions::new().set_enforce_schema(true).build());
```

:::note
Pin Schemas are not stored inline with the node. These are classical JSON Schema definitions, which can be quite huge. 
We are hashing these Schemars for all nodes and storing them in the references of the board. Depending on the Size of the Struct you are working with this can however still add quite some overhead.
:::


### Dynamic Pin Amount
As you might have seen already, there is the option to allow an arbitrary amount of input or output pins. This can be achieved by defining the same Pin Name multiple times. Users can than add more Pins of this type to the node in the frontend. The minimum number of Pins in this case is however 2.

```rust title="And Node" {7} {15}
async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new("bool_and", "And", "Boolean And operation", "Utils/Bool");

        node.add_icon("/flow/icons/bool.svg");

        node.add_input_pin(
            "boolean",
            "Boolean",
            "Input Pin for AND Operation",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_input_pin(
            "boolean",
            "Boolean",
            "Input Pin for AND Operation",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_output_pin(
            "result",
            "Result",
            "AND operation between all boolean inputs",
            VariableType::Boolean,
        );

        return node;
    }
```