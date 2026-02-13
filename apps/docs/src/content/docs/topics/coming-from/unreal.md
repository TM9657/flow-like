---
title: For Unreal Engine Developers
description: How Unreal Blueprints concepts translate to Flow-Like
sidebar:
  order: 5
---

Coming from **Unreal Engine Blueprints**? You already understand visual programming! This guide maps Blueprint concepts to Flow-Like, helping you apply your node graph skills to automation and AI workflows.

## Quick Concept Mapping

| Blueprint Concept | Flow-Like Equivalent |
|-------------------|---------------------|
| Blueprint | Board |
| Event Graph | Flow |
| Node | Node |
| Pin | Pin |
| Execution Pin (white) | Execution Wire |
| Data Pin (colored) | Data Wire |
| Variable | Variable |
| Function | Board with Quick Action |
| Macro | Subflow / Board reference |
| Event | Event node |
| Cast To | Type conversion nodes |
| Branch | Branch node |
| For Each Loop | For Each node |
| Sequence | Multiple output wires |
| Struct | Struct type |
| Array | Array type |
| Pure Function | Pure nodes (no execution pin) |
| Reroute Node | Reroute (visual organization) |

## The Familiar Visual Model

If you've used Blueprints, Flow-Like will feel natural:

**Blueprint Event Graph:**
```
(Event BeginPlay) ──▶ [Print String] ──▶ [Set Variable]
```

**Flow-Like Flow:**
```
[Quick Action Event] ──▶ [Console Log] ──▶ [Set Variable]
```

Both use:
- Left-to-right execution
- Nodes connected by wires
- Input pins on left, output pins on right
- Execution flow (white wires) and data flow (colored wires)

## Execution Wires

Just like Blueprints, Flow-Like has **execution wires** (white) that control flow order:

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│  Event          │     │  Process        │     │  Save           │
│            exec ├────▶│ exec       exec ├────▶│ exec            │
│                 │     │                 │     │                 │
└─────────────────┘     └─────────────────┘     └─────────────────┘
```

Nodes execute in the order the execution wire connects them.

## Data Wires

Data pins work identically:

**Blueprint:**
```
[Get Player Location] ──▶ (Vector) ──▶ [Print String]
```

**Flow-Like:**
```
[Get Variable: location] ──▶ (Vector3) ──▶ [Console Log]
```

Colored wires carry data. Types must match (or be convertible).

## Variables

### Blueprint Variables

In Blueprints, variables are scoped to the Blueprint class.

### Flow-Like Variables

Variables are scoped to the Board:
```
Board: MyWorkflow
├── Variables:
│   ├── counter: Integer = 0
│   ├── playerData: PlayerInfo
│   └── items: Array<Item>
```

Access with Get/Set Variable nodes—exactly like Blueprints.

## Events

| Blueprint Event | Flow-Like Event |
|-----------------|-----------------|
| Event BeginPlay | Init Event (if applicable) |
| Event Tick | Scheduled Event |
| Custom Event | Quick Action Event |
| Event Dispatcher | Quick Action (callable) |
| Input Event | (not applicable—no game input) |
| Collision/Overlap | (not applicable—no physics) |

Flow-Like events are triggers for workflows:
- **Quick Action** – Manual button click
- **Chat Event** – Conversational input
- **HTTP Event** – API webhook
- **Scheduled Event** – Timer-based

## Control Flow

### Branch (If)

**Blueprint:**
```
[Branch]
├── Condition ──▶
├── True ──▶ [Do Something]
└── False ──▶ [Do Other]
```

**Flow-Like:**
```
[Branch]
├── condition ◀── (bool input)
├── True ──▶ [Do Something]
└── False ──▶ [Do Other]
```

Identical pattern!

### For Each Loop

**Blueprint:**
```
[For Each Loop]
├── Array ◀── (array input)
├── Loop Body ──▶ [Process]
│   ├── Array Element ──▶
│   └── Array Index ──▶
└── Completed ──▶ [After Loop]
```

**Flow-Like:**
```
[For Each]
├── array ◀── (array input)
├── body ──▶ [Process]
│   ├── element ──▶
│   └── index ──▶
└── done ──▶ [After Loop]
```

Same structure, same semantics.

### Sequence

**Blueprint:**
```
[Sequence]
├── Then 0 ──▶ [First]
├── Then 1 ──▶ [Second]
└── Then 2 ──▶ [Third]
```

**Flow-Like:**
Simply connect multiple wires from one node:
```
[Event] ──┬──▶ [First]
          ├──▶ [Second]
          └──▶ [Third]
```

Branches execute in parallel (unlike Blueprint's sequential).

### Flip Flop / Do Once

**Blueprint:** Built-in nodes like Flip Flop.

**Flow-Like:** Use variables to track state:
```
[Get Variable: flip_state]
    │
    ▼
[Branch: flip_state == true]
    │
   True ──▶ [Action A] ──▶ [Set Variable: flip_state = false]
    │
   False ──▶ [Action B] ──▶ [Set Variable: flip_state = true]
```

## Functions & Macros

### Blueprint Functions → Boards with Quick Actions

**Blueprint Function:**
```
Function: CalculateScore
├── Inputs: kills, deaths
├── Local Variables: ratio
└── Return: score

[Divide] ──▶ [Multiply] ──▶ [Return Node]
```

**Flow-Like:**
```
Board: CalculateScore
├── Quick Action Event:
│   ├── kills (input)
│   └── deaths (input)
└── Flow:
    [Divide] ──▶ [Multiply] ──▶ [Return]
```

Call from another Board just like calling a Blueprint function.

### Macros → Subflows

Blueprint Macros expand inline. In Flow-Like, use subflows or board references for reusable logic.

## Pure Functions

**Blueprint Pure Nodes:** No execution pins, just data.

**Flow-Like Getter Nodes:** Same concept:
```
[Get Variable: score] ──▶ (value)  // Pure, no exec wire
```

Pure nodes can be connected to multiple consumers and will evaluate when needed.

## Casting

**Blueprint:**
```
[Cast To PlayerCharacter]
├── Object ◀──
├── Success ──▶ [Use as PlayerCharacter]
└── Failed ──▶ [Handle Error]
```

**Flow-Like:**
Use type-specific nodes or validation:
```
[Validate Type]
├── value ◀──
├── valid ──▶ [Use Value]
└── invalid ──▶ [Handle Error]
```

Or Extract Knowledge with schema validation for structured data.

## Structs

**Blueprint Struct:**
```
Struct: S_PlayerData
├── Name: String
├── Score: Integer
└── Inventory: Array<S_Item>
```

**Flow-Like Struct:**
```
Struct: PlayerData
├── name: String
├── score: Integer
└── inventory: Array<Item>
```

Break/Make struct nodes work similarly:
```
[Make PlayerData]
├── name ◀── "Alice"
├── score ◀── 100
├── inventory ◀── [empty array]
└── ──▶ PlayerData instance
```

```
[Get Field: name]
├── struct ◀── playerData
└── ──▶ "Alice"
```

## Arrays

Array operations are nearly identical:

| Blueprint Node | Flow-Like Node |
|----------------|----------------|
| Make Array | Create Array |
| Add | Append |
| Insert | Insert |
| Remove Index | Remove at Index |
| Remove Item | Remove Item |
| Get | Get at Index |
| Length | Array Length |
| Find | Find Index |
| Contains | Contains |
| Filter | Filter Array |
| Set Array Elem | Set at Index |
| Append Array | Concat Arrays |

## Math & Operations

All familiar math nodes exist:
- Add, Subtract, Multiply, Divide
- Sin, Cos, Tan, etc.
- Clamp, Lerp, Map Range
- Min, Max, Abs
- Vector operations

## String Operations

| Blueprint | Flow-Like |
|-----------|-----------|
| Append | Concat |
| Format Text | Template String |
| To String | Stringify |
| Contains | String Contains |
| Split | Split String |
| Join | Join Strings |
| Replace | String Replace |
| To Upper/Lower | To Uppercase / To Lowercase |

## Comparison: Game vs. Automation

| Blueprint Use Case | Flow-Like Equivalent |
|--------------------|---------------------|
| Player spawns | Quick Action triggered |
| Game tick | Scheduled event |
| Button pressed | Chat Event / Quick Action |
| API call | HTTP Request |
| Save game | Save to Database / File |
| AI behavior tree | Agent nodes |
| UI update | A2UI components |
| Network replicate | (not applicable) |

## What's Different

### No Real-Time Execution
Blueprints run every frame. Flow-Like runs on-demand (events trigger flows).

### No Game Objects
No Actors, Components, or World. Instead: files, APIs, databases, AI.

### No Physics/Collision
Flow-Like is for data processing, not simulation.


## What's Similar

### Visual Debugging
- Blueprints: Execution trace, watch values
- Flow-Like: Wire inspection, execution history

### Type System
Both enforce types at connection time. Incompatible types can't connect.

### Modular Design
- Blueprints: Functions, Macros, Child Blueprints
- Flow-Like: Boards, Quick Actions, Board references

## What Flow-Like Adds

### AI & LLMs

Native AI integration:
```
[Chat Event] ──▶ [Invoke LLM] ──▶ [Response]
```

Build conversational AI, agents, RAG systems.

### Data Processing

SQL across any source:
```
[Register CSV] ──▶ [SQL Query] ──▶ [Results Table]
```

### Integrations

Connect to real-world services:
- REST APIs
- Databases (PostgreSQL, MySQL, etc.)
- Cloud storage (S3, Azure, GCS)
- File systems

### Deployment

Run workflows:
- Desktop app (like a packaged game)
- Cloud backends (like dedicated servers)
- Scheduled (like background services)

## Example: Blueprint to Flow-Like

### Blueprint: Score Tracker

```
Event BeginPlay
    │
    ▼
Set Score = 0
    │
    ▼
Bind Event: OnEnemyKilled → Add to Score

---

Function: AddToScore(points)
├── Get Score
├── Add (Score + points)
├── Set Score
└── Update UI
```

### Flow-Like: Task Tracker

```
Board: TaskTracker
├── Variables:
│   ├── completed_count: Integer = 0
│   └── tasks: Array<Task>
│
└── Events:
    ├── Quick Action: AddTask (task_name)
    │       │
    │       ▼
    │   [Create Task] ──▶ [Append to tasks] ──▶ [Set Variable]
    │
    └── Quick Action: CompleteTask (task_id)
            │
            ▼
        [Find Task] ──▶ [Mark Complete] ──▶ [Increment completed_count]
                                               │
                                               ▼
                                        [Update UI Log]
```

### Blueprint: AI Patrol

```
Event Tick
    │
    ▼
Get Next Patrol Point
    │
    ▼
Move To Location
    │
    ▼
Branch: At Location?
├── True ──▶ Wait 2s ──▶ Get Next Point
└── False ──▶ Continue Moving
```

### Flow-Like: Data Monitor

```
Scheduled Event (every 5 minutes)
    │
    ▼
HTTP Request: Get Metrics
    │
    ▼
Branch: Metric > Threshold?
├── True ──▶ Send Alert (Slack)
│               │
│               ▼
│           Log to Database
│
└── False ──▶ Log: "All normal"
```

## Tips for Blueprint Developers

### 1. Think Events, Not Ticks
Replace constant polling with event-driven triggers.

### 2. Use Variables for Persistence
Your "game state" is Board Variables.

### 3. Boards Are Blueprints
Each Board is like a Blueprint class—self-contained logic unit.

### 4. Quick Actions Are Custom Events
Expose functionality that other boards (or users) can call.

### 5. Data Flow Is Familiar
Same pins, same wires, same left-to-right flow.

## FAQ

### Can I use this for game development?
Flow-Like is for automation/AI, not games. But the skills transfer!

### Is there a marketplace?
Flow-Like has packages. Community contributions work similarly.

### Can I prototype game logic?
Yes—for data flow and AI behavior (not rendering/physics).

### Does it work with Unreal?
You could trigger Flow-Like workflows from Unreal via HTTP, but they're separate tools.

## Next Steps

- **[Studio Overview](/studio/overview/)** – Learn the IDE
- **[Working with Nodes](/studio/nodes/)** – Node deep dive
- **[Variables](/studio/variables/)** – State management
- **[GenAI](/topics/genai/overview/)** – Build AI with familiar node graphs
- **[Agents](/topics/genai/agents/)** – AI that feels like Behavior Trees
