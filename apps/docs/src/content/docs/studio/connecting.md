---
title: Connections
description: Typed Connections / Wires between Nodes
sidebar:
    order: 25
---

### Connection Types

There are two types of *wires* / *connections* between nodes:
- **Execution Wires** (*white*) represent execution transmission throughout the flow graph, typically starting with an *event node*. Executions can take turns at *Branch Nodes*, repeat in *Loop Nodes* or split up for *parallel* execution.
- **Data Wires** (*colored, dashed*) represent data transmission between nodes. The *color* of a data wire represents the *data type* (see also [Variables and Types](/studio/variables/)).

All pins in FlowLike Studio *enfore types*:
- You can only wire execution pins to execution pins.
- You can only wire data pins to those of the *same type* (aka *color*). 

Some nodes additionally enforce a *schema* on complex types (structs, *purple*). For example, a *Path* output is only accepted by those nodes also having a *Path* input pin.

![A screenshot showing different wire / connection types in FlowLike Studio](../../../assets/ConnectionsWires.webp)

Some nodes come with *generic (unspecified) types* when selected from the node catalog. For example, the *For Node* allows to loop over arrays of different types but once an upstream data pin is connected, its type is *fixed* (e.g. a *For Node* for *Paths*):

![A screenshot showing how an upstream data pin sets the type of a generically typed input pin](../../../assets/GenericPinTypes.webp)


### Auto-Suggestions Based on Types

Thanks to FlowLike's strong typing mechanism, we can exploit the fact that only pins of the same type can be wired and suggest matching nodes.

Drag a pin (input or output) into the open canvas to create a new wired node that will be immediatedly connected to the current node:

![A screenshot showing how to drag a node pin into the open canvas to immediatedly create a new node + wire.](../../../assets/DrawPin.webp)

Once you drop the dragged wired, the node catalog dialog opens and suggests only those nodes that you can actually connect to this pin:

![A screenshot showing how the node catalog is reduced to the set of nodes that can actually be connected to the selected pin](../../../assets/TypedCatalogSuggestions.webp)

Catalog filtering based on *types* can signicantly speed up your flow creation process.