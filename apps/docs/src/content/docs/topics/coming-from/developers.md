---
title: For Developers
description: Translating programming concepts to Flow-Like's visual paradigm
sidebar:
  order: 3
---

Coming from a **traditional coding background**? This guide translates familiar programming concepts to Flow-Like's visual workflow approach. Whether you're a Python dev, JavaScript engineer, or seasoned systems programmer, you'll find the mental models transfer well.

## The Core Paradigm Shift

Instead of writing code like:
```javascript
const result = processData(loadFile(path));
saveOutput(result, outputPath);
```

You build visual pipelines:
```
Load File ──▶ Process Data ──▶ Save Output
```

The flow is the program. Nodes are functions. Wires are data passing.

## Quick Concept Mapping

| Programming Concept | Flow-Like Equivalent |
|--------------------|---------------------|
| Function | Node |
| Function call | Connect node output to input |
| Variable | Variable (scoped to Board) |
| Parameter | Input Pin (left side of node) |
| Return value | Output Pin (right side of node) |
| Module/Class | Board |
| Import | Reference another Board |
| Main function | Event (entry point) |
| Loop | For Each / While nodes |
| Conditional | Branch node |
| Try/Catch | Try + Catch nodes |
| Type | Pin type |
| Struct/Object | Struct type |
| Array | Array type |
| Callback | Sub-flows |
| Async/await | Automatic (execution pauses) |
| Thread | Parallel branches |

## Functions → Nodes

Every node is essentially a function call.

**Code:**
```python
def add(a: int, b: int) -> int:
    return a + b

result = add(5, 3)
```

**Flow-Like:**
```
┌─────────────────┐
│      Add        │
│  5 ──▶ a        │
│  3 ──▶ b        │
│         sum ──▶ │ 8
└─────────────────┘
```

The inputs (a, b) are left-side pins. The output (sum) is a right-side pin.

### Chaining Functions

**Code:**
```python
value = step3(step2(step1(input)))
```

**Flow-Like:**
```
Input ──▶ Step 1 ──▶ Step 2 ──▶ Step 3 ──▶ Output
```

Data flows left-to-right through connected pins.

## Variables

Variables in Flow-Like are scoped to Boards (like class attributes):

**Code:**
```python
class MyProcessor:
    def __init__(self):
        self.counter = 0
        self.results = []

    def process(self, item):
        self.counter += 1
        self.results.append(item)
```

**Flow-Like:**
```
Board Variables:
├── counter: Integer (default: 0)
└── results: Array<Item>

Event: Process (item)
    │
    ├──▶ Get Variable: counter
    │         │
    │         ▼
    │    Add (counter, 1)
    │         │
    │         ▼
    │    Set Variable: counter
    │
    └──▶ Get Variable: results
              │
              ▼
         Append (results, item)
              │
              ▼
         Set Variable: results
```

### Variable Operations

| Operation | Nodes |
|-----------|-------|
| Read | Get Variable |
| Write | Set Variable |
| Modify | Get → Transform → Set |

## Control Flow

### If/Else → Branch

**Code:**
```python
if condition:
    do_a()
else:
    do_b()
```

**Flow-Like:**
```
              True ──▶ Do A
Condition ──▶ Branch ─┤
              False ──▶ Do B
```

### Switch/Match → Multiple Branches

**Code:**
```python
match status:
    case "pending":
        handle_pending()
    case "approved":
        handle_approved()
    case "rejected":
        handle_rejected()
```

**Flow-Like:**
```
                    ┌──▶ "pending" ──▶ Handle Pending
Get Status ──▶ Switch ├──▶ "approved" ──▶ Handle Approved
                    └──▶ "rejected" ──▶ Handle Rejected
```

### For Loops → For Each

**Code:**
```python
for item in items:
    process(item)
```

**Flow-Like:**
```
Items ──▶ For Each ──▶ Process (item) ──▶ Continue Loop
              │
              └──▶ (done) ──▶ Next Step
```

### While Loops

**Code:**
```python
while condition:
    do_work()
    update_condition()
```

**Flow-Like:**
```
┌─────────────────────────────────────┐
│                                     │
│  ┌──────────────────┐               │
│  │                  │               │
└──┤ While (condition)├──▶ Do Work ──┤
   │                  │               │
   └────False─────────┼───────────────┘
                      │
                      ▼
                 Next Step
```

## Error Handling

**Code:**
```python
try:
    result = risky_operation()
except SpecificError as e:
    handle_error(e)
finally:
    cleanup()
```

**Flow-Like:**
```
Try ──▶ Risky Operation ──▶ Continue
 │
 └──Catch ──▶ Handle Error
           │
           └──▶ (always runs) ──▶ Cleanup
```

## Types & Structs

Flow-Like is strongly typed. Define structures for complex data:

**Code:**
```typescript
interface User {
  id: string;
  name: string;
  email: string;
  orders: Order[];
}
```

**Flow-Like:**
```
Struct: User
├── id: String
├── name: String
├── email: String
└── orders: Array<Order>
```

### Working with Structs

**Code:**
```python
user.name = "Alice"
email = user.email
```

**Flow-Like:**
```
Set Field (user, "name", "Alice") ──▶ updated_user

Get Field (user, "email") ──▶ email_value
```

## Async & Parallelism

### Sequential (Await)

**Code:**
```python
result1 = await step1()
result2 = await step2(result1)
```

**Flow-Like:**
```
Step 1 ──▶ Step 2 ──▶ Done
```

Execution automatically waits. No async/await syntax needed.

### Parallel Execution

**Code:**
```python
results = await asyncio.gather(
    task1(),
    task2(),
    task3()
)
```

**Flow-Like:**
```
           ┌──▶ Task 1 ──┐
Start ──▶ Split          ├──▶ Merge ──▶ Combined Results
           ├──▶ Task 2 ──┤
           └──▶ Task 3 ──┘
```

Branches without dependencies execute in parallel automatically.

## Modules & Imports

**Code:**
```python
from utils import helper_function

result = helper_function(data)
```

**Flow-Like:**
```
Board: Utils
└── Event: HelperFunction (data)
        │
        ▼
    Process ──▶ Return Result

Board: Main
└── Event: Process
        │
        ▼
    Call Board: Utils.HelperFunction (data)
        │
        ▼
    Use Result
```

Boards are your modules. Quick Actions are your exported functions.

## Common Patterns

### Map/Transform

**Code:**
```python
processed = [transform(item) for item in items]
```

**Flow-Like:**
```
Items ──▶ For Each ──▶ Transform ──▶ Collect ──▶ Processed Array
```

### Filter

**Code:**
```python
filtered = [item for item in items if condition(item)]
```

**Flow-Like:**
```
Items ──▶ For Each ──▶ Branch (condition)
                          │
                     True │
                          ▼
                      Collect ──▶ Filtered Array
```

### Reduce/Aggregate

**Code:**
```python
total = sum(item.value for item in items)
```

**Flow-Like:**
```
Variables: running_total = 0

Items ──▶ For Each ──▶ Get Value ──▶ Add to running_total
              │
              └──(done)──▶ Get running_total ──▶ Final Total
```

### HTTP Client

**Code:**
```python
response = requests.post(
    "https://api.example.com/data",
    json={"key": "value"},
    headers={"Authorization": "Bearer token"}
)
data = response.json()
```

**Flow-Like:**
```
HTTP Request
├── URL: "https://api.example.com/data"
├── Method: POST
├── Body: {"key": "value"}
└── Headers: {"Authorization": "Bearer token"}
    │
    ▼
Parse JSON ──▶ data
```

### File I/O

**Code:**
```python
with open("file.txt", "r") as f:
    content = f.read()

with open("output.txt", "w") as f:
    f.write(processed_content)
```

**Flow-Like:**
```
Read to String ("file.txt") ──▶ content
                                    │
                                    ▼
                              Process Content
                                    │
                                    ▼
Write String ("output.txt", processed_content)
```

### Database Queries

**Code:**
```python
conn = psycopg2.connect(...)
cursor = conn.cursor()
cursor.execute("SELECT * FROM users WHERE active = true")
users = cursor.fetchall()
```

**Flow-Like:**
```
Register PostgreSQL (connection_string)
    │
    ▼
SQL Query ("SELECT * FROM users WHERE active = true")
    │
    ▼
users (array of rows)
```

## Debugging

| Programming | Flow-Like |
|-------------|-----------|
| `print(variable)` | Console Log node |
| Breakpoint | Pause execution (click wire) |
| Step through | Visual execution trace |
| Stack trace | Follow execution path |
| Watch variables | Inspect any pin value |

### Debug Mode

1. Run your flow
2. Click any wire to see its current value
3. Errors show red highlighting on the failing node
4. Execution history shows the path taken

## Testing

**Code:**
```python
def test_add():
    assert add(2, 3) == 5
    assert add(-1, 1) == 0
```

**Flow-Like:**
```
Board: TestAdd
├── Test Case 1:
│   └── Add(2, 3) ──▶ Assert Equals (5)
│
└── Test Case 2:
    └── Add(-1, 1) ──▶ Assert Equals (0)
```

Run test boards to validate logic.

## Performance Considerations

### What's Fast
- Node execution (Rust runtime)
- Data passing (zero-copy where possible)
- Parallel branches (truly concurrent)
- Native operations (files, HTTP, SQL)

### What to Optimize
- Minimize node count in hot paths
- Use batch operations over loops when available
- Leverage SQL for data filtering (don't load all data)
- Cache expensive computations in variables

## Creating Custom Nodes

When built-in nodes aren't enough, create custom ones:

### WASM Nodes (Rust/AssemblyScript)
```rust
#[wasm_bindgen]
pub fn my_custom_function(input: String) -> String {
    // Your logic here
    format!("Processed: {}", input)
}
```

This becomes a node you can use in any flow.

## Code Integration

### Calling External APIs
Use HTTP Request nodes to call any REST API.

### Running Scripts
Use the Run Command node to execute shell commands.

### Embedding in Apps
Flow-Like flows can be triggered via API endpoints.

## FAQ

### Can I write code instead?
Some complex logic may require custom WASM nodes. But most automations work visually.

### Is it slower than code?
The runtime is Rust—often faster than Python/JS. The overhead is negligible.

### How do I version control?
Flow-Like has built-in versioning. Boards also export as JSON for Git.

### Can I collaborate?
Yes—share boards, use version history, export/import packages.

### What about code review?
Visual diffs show what changed. It's different but reviewable.

## Mental Model Tips

### 1. Think Data Flow
Code executes line-by-line. Flows execute node-by-node following wires.

### 2. Nodes Are Pure (Mostly)
Nodes take inputs, produce outputs. Side effects are explicit (file writes, HTTP calls).

### 3. Variables Are State
When you need persistent state, use variables. They're like class attributes.

### 4. Boards Are Boundaries
Each board is a unit of composition. Like modules or classes.

### 5. Events Are Entry Points
Nothing runs without an event trigger. They're your main() functions.

## What You Gain

| Pain Point in Code | Flow-Like Solution |
|--------------------|--------------------|
| Dependency management | Bundled in nodes |
| Environment setup | Just download and run |
| Deployment complexity | Click to publish |
| Documentation | Visual is self-documenting |
| Onboarding teammates | Lower barrier |
| Debugging async flows | Visual trace |

## Next Steps

- **[Studio Overview](/studio/overview/)** – Learn the IDE
- **[Working with Nodes](/studio/nodes/)** – Node deep dive
- **[Variables](/studio/variables/)** – State management
- **[Events](/apps/events/)** – Entry points and triggers
- **[GenAI](/topics/genai/overview/)** – AI capabilities
