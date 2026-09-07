---
title: Customize your Home
description: Arrange Home widgets, save a personal layout, and choose which defaults to inherit.
---

Home brings your Apps, FlowPilot, workspace activity, and discovery widgets into
one page. Choose **Customize** to add widgets, change their content, or arrange
them for the active Profile. Web and desktop use the same editor.

## Save a personal layout

1. Select the Profile whose Home you want to change.
2. Open **Customize**, then **Add widget** to choose a widget. Select an existing
   widget to edit its content, data source, size, or appearance.
3. Drag widgets into position. With the keyboard, pick up a widget using its
   move handle, use the arrow keys to choose a position, and press Space to
   place it. Escape cancels the move or an active pointer resize.
4. Choose **Save** to apply the layout. **Cancel** lets you discard unsaved
   changes. Undo and redo are available while editing.

Finish an active move or resize before saving. If a save fails, the editor
retains the draft so you can retry.

## Understand defaults

Home uses the first available layout in this order:

1. Your saved personal layout for the active Profile.
2. The published default assigned to that Profile.
3. The installation's published main default.
4. The starter bundled with Flow-Like.

A personal layout takes precedence over later changes to published defaults.
To return to inherited defaults, open **Customize → Layout options → Reset to
default**, confirm the reset, then save. You can undo the reset before saving.

To start from the current bundled design instead, choose **Use Flow-Like
starter** in the same menu. This loads an editable draft. It does not change
your saved layout until you save, and it does not publish an installation-wide
default. Administrators manage shared defaults separately; see
[platform administration](/dev/platform-administration/).

When you use an inherited layout and its published default cannot be loaded,
Home uses its last available cached default or the bundled starter. A failure to
load the Profile itself shows a retry action. Check that the intended Profile is
active before retrying a save.

## Size widgets for their content

**Match row** aligns a widget's surface with its neighbors. **Fit content**
keeps its natural height. Choose a row count or resize the widget when you need
a fixed height. On narrow screens, widgets form a single column.

The widget's **surface** controls its frame and accent. Collection **card
style** controls how the Apps, models, or packages inside it are displayed.
These are separate choices. For manual App collections, you can reorder and
remove selected Apps; automatic collection filters do not override that manual
selection.

Embedded Apps keep their own page navigation and query parameters. Their
preview pauses while you edit Home. Account activity widgets report the
available account execution records; an unavailable data source should be
retried rather than interpreted as zero activity.
