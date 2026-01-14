# A2UI Frontend Builder - External AI Setup

This directory contains system prompts and knowledge files for creating Gemini Gems and ChatGPT Custom GPTs that can generate A2UI JSON interfaces.

## Try It Now

**ChatGPT GPT:** [FlowLike Frontend Builder](https://chatgpt.com/g/g-6965146c7f5c81918a2501c5a860d9e3-flow-like-frontend-builder)

## Styling Guidelines

The generators use shadcn/ui theme tokens for automatic dark/light mode support:
- **Preferred:** `bg-background`, `text-foreground`, `bg-primary`, `text-muted-foreground`, etc.
- **Also allowed:** Hardcoded Tailwind colors (`bg-red-500`, `text-blue-600`) when users explicitly request specific colors

Theme tokens are preferred because they adapt to the user's color scheme, but specific color requests should be honored.

## Directory Structure

```
frontend-builder/
├── gemini/
│   └── system.md          # System prompt for Gemini Gem
├── chatgpt/
│   └── system.md          # System instructions for ChatGPT GPT
└── docs/
    ├── components-reference.md   # Full component documentation
    ├── bound-value-guide.md      # Data binding patterns
    ├── styling-guide.md          # Tailwind/shadcn rules
    └── layout-examples.md        # Complete JSON examples
```

## Setup Instructions

### Gemini Gem

1. Go to [Google AI Studio](https://aistudio.google.com/) → Gems
2. Create a new Gem
3. Copy the contents of `gemini/system.md` into the **System Prompt**
4. Upload the files from `docs/` as **Knowledge** files
5. Test with prompts like "Create a login form" or "Build a dashboard with stats cards"

### ChatGPT Custom GPT

1. Go to [ChatGPT](https://chat.openai.com/) → Explore GPTs → Create
2. Copy the contents of `chatgpt/system.md` into the **Instructions**
3. Upload the files from `docs/` as **Knowledge** files
4. Enable "Code Interpreter" for JSON validation (optional)
5. Test with similar prompts

## Knowledge Files

The docs files are designed to be uploaded as knowledge/context:

| File | Purpose | Upload Priority |
|------|---------|-----------------|
| `components-reference.md` | All component props | **Required** |
| `bound-value-guide.md` | Data binding patterns | Recommended |
| `styling-guide.md` | Tailwind/theme rules | Recommended |
| `layout-examples.md` | Full JSON examples | Optional (helps quality) |

## Usage Tips

1. **Be specific** - "Create a login form with email and password fields" works better than "make a form"
2. **Mention layout** - "Create a 3-column grid of feature cards" helps the AI understand structure
3. **Data binding** - Mention if you need dynamic data: "Show user name from $.user.name"
4. **Copy the JSON** - The output can be pasted directly into FlowLike's page builder

## Example Prompts

- "Create a pricing page with 3 tier cards (Free, Pro, Enterprise)"
- "Build a user profile card with avatar, name, email, and edit button"
- "Make a dashboard with 4 stat cards and a line chart below"
- "Create a settings form with toggles for notifications, dark mode, and email preferences"
- "Build a product listing grid that binds to $.products data"

## Output Format

The AI will generate JSON like this:

```json
{
  "rootComponentId": "main",
  "canvasSettings": { "backgroundColor": "bg-background", "padding": "1rem" },
  "components": [
    { "id": "main", "style": { "className": "..." }, "component": { "type": "column", ... } }
  ]
}
```

This JSON can be imported directly into FlowLike's A2UI page builder.
