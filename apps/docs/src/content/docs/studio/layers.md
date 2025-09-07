---
title: Layers and Placeholders
description: Organize and Prototype Flows with Layers
sidebar:
  order: 50
---

*Layers* and *Playholders* are a powerful feature of FlowLike Studio for abstraction and prototyping in your automations. They refer to the same functionality: creating nested or collapsed flows *within* flows. You can also think of them as "third dimension" in your boards.

## Collapsing Existing Nodes into New Layers

At some point in your flow creation process you might want to "summarize" nodes either because you need certain combinations mulitple times or because the canvas becomes too cluttered.

To do so, you can select two or more nodes and *collapse* them into a *Layer*:

![A screenshot showing how to collapse multipe nodes into one placeholder node](../../../assets/CollapsingNodes.webp)

The resulting collapsed *Layer* is represented by a *Placeholder* node that we can now rename to describe what's happening inside:

![A screenshot showing how to rename a collapsed node](../../../assets/SetLayerName.webp)

Once created, we can also *edit* the input and output pins of a layer to either change the type or to change the pin names:

![A screenshot showing how to add, remove, rename, or change the the type of pins for placholder nodes](../../../assets/EditingLayerPins.webp)

Clicking on a *layer* / *placeholder node* allows us to navigate inside. Here you can see the part of the flow we've previously collapsed. Inputs and outputs connecting the inside to the outside are represented by *start* and *return* nodes:

![A screenshot showing how to navigate inside and outside of a layered node](../../../assets/InsideLayers.webp)

Collapsing nodes into layers allows you to create meaningful abstractions of your flow graph such that it effectively represents the core logic of your application on the top layer:

![A screenshot showing how collapsed nodes can help to make the core application logic explicit on the top level](../../../assets/FlowWithLayers.webp)

Note that collapsed nodes have all properties of standard nodes concerning typing, wiring, copying etc. You can even collapse them again creating further levels of abstraction.

## Prototype with Placeholders

*Placeholder* nodes are *layers* that are simply *empty*. This allows you to quickly prototype your application logic and implement it *later*.

You can select placeholder nodes from the node catalog and then rename or edit their pins as described above:

![A screenshot showing how to select an empty placeholder node](../../../assets/PlaceholdersForPrototyping.webp)

With placeholders you can visually design your application, maybe with some first execution connections or pin types defined. Once you feel confident that the overall layout works out you can start implementing each placeholder.