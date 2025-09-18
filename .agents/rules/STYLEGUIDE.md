# BLZ Style Guide

## Core Principle: Honest & Humble

We write with clarity and honesty. We're building tools for developers who value substance over hype. Our tone reflects confidence through capability, not volume.

## Voice & Tone

### What We Are

- **Honest**: State capabilities accurately. "6ms search" not "BLAZINGLY ULTRA-FAST!!!"
- **Humble**: Let the tool speak for itself. Results over rhetoric.
- **Professional**: Clear, concise, technical when needed
- **Helpful**: Focus on solving real problems
- **Clever** (sparingly): Occasional wordplay is fine, but utility comes first

### What We're Not

- **Braggadocious**: No "revolutionary", "game-changing", "world's best"
- **Hyperbolic**: Avoid superlatives unless factually accurate
- **Loud**: No ALL CAPS for emphasis, no excessive exclamation points
- **Sales-y**: We're not selling, we're providing tools

## Writing Guidelines

### Product References

#### When to Use "BLZ" (Uppercase)
Use uppercase "BLZ" when referring to the project or tool in prose:
- Documentation headers and titles
- README descriptions
- Inline comments in code
- CLI help strings and descriptions
- General references in documentation text

Examples:
- "BLZ is a local-first search cache"
- "Welcome to BLZ"
- "What is BLZ?"
- "BLZ indexes your documentation"

#### When to Use "blz" (Lowercase)
Keep lowercase `blz` for technical/code contexts:
- CLI commands and examples: `blz add`, `blz search`
- File paths: `/usr/local/bin/blz`
- Package/crate names: `@outfitter/blz`, `blz-core`
- Code identifiers and variables
- URLs and domains
- Inside code blocks or backticks

Examples:
```bash
blz add react https://react.dev/llms.txt
cargo install blz-cli
```

#### Pronunciation
- Pronounced "blaze" but don't overexplain this
- Don't overuse the blazing/fire metaphors

### Performance Claims
✅ **Good**: "6ms typical search latency"
❌ **Bad**: "INSANELY FAST searches that will BLOW YOUR MIND"

✅ **Good**: "Returns results in milliseconds"
❌ **Bad**: "Lightning-fast, blazingly quick, supersonic speed!!!"

### Feature Descriptions
✅ **Good**: "Local-first search that works offline"
❌ **Bad**: "Revolutionary offline-first architecture"

✅ **Good**: "Indexes llms.txt documentation for fast retrieval"
❌ **Bad**: "The ULTIMATE documentation indexing solution"

### Error Messages
✅ **Good**: "Source not found. Run `blz add <alias> <url>` to add it."
❌ **Bad**: "Oops! 😱 We couldn't find that source! 🚨"

## Emoji Usage

### When to Use

- **Sparingly**: One per paragraph maximum
- **Functionally**: To aid scanning or categorization
- **Consistently**: Same emoji for same meaning

### Approved Emojis

- 🔥 **Speed/Performance** - Use rarely, only when emphasizing actual speed metrics
- 🔹 **Trail marker/waypoint** - For navigation, guidance, shortcuts (blue blazing)
- 🔷 **Alternative trail marker** - Same usage as 🔹
- ✅ **Success/Correct** - For confirmation messages
- ❌ **Error/Incorrect** - For error states
- 💡 **Tip/Hint** - For helpful suggestions

### Emoji Guidelines

- Never use more than one emoji in a row
- Don't use emojis in error messages that users will see frequently
- Prefer text over emoji when clarity matters

## Metaphor Usage

### Trail Blazing
Use the trail blazing metaphor to explain navigation and discovery:

- "Mark your path through documentation"
- "Guide through the docs wilderness"
- "Find shortcuts to answers" (blue blazing)

Don't overdo it:

- ❌ "Blaze an epic trail through the documentation forest!"
- ❌ "Become a master trail blazer!"

### Speed
Reference speed with specific metrics:

- ✅ "Search in 6ms"
- ✅ "Millisecond response times"
- ❌ "Blazingly fast"
- ❌ "Lightning speed"

## Documentation Standards

### README Structure

1. **Definition**: Dictionary-style, three-part definition
2. **One-liner**: Simple, factual description
3. **Features**: Bullet points with specific capabilities
4. **Quick Start**: Immediate value, minimal steps
5. **Details**: Technical information for those who need it

### Command Examples

```bash
# Good: Clear, practical examples
blz add react https://react.dev/llms.txt
blz "useEffect cleanup"

# Bad: Trying too hard
blz add react https://react.dev/llms.txt --awesome --fire 🔥
```

### Headings

- Use sentence case: "Quick start guide" not "Quick Start Guide"
- Be descriptive: "Search syntax" not "Advanced Features"
- No emoji in headings

## Marketing Copy

### Taglines
✅ **Good**:

- "Fast local search for llms.txt documentation"
- "Your agent's guide through the docs wilderness"
- "6ms to any answer"

❌ **Bad**:

- "The FASTEST documentation tool EVER CREATED!"
- "🔥🔥🔥 BLAZING FAST SEARCH 🔥🔥🔥"
- "Revolutionary game-changing documentation paradigm"

### Feature Highlights
Focus on measurable benefits:

- "6ms search latency"
- "Works offline"
- "Exact line citations"

Not vague superlatives:

- "Ultra-fast"
- "Best-in-class"
- "Unparalleled performance"

## Code Comments

### In-Code Documentation

```rust
// Good: Explains the why
// Use BM25 for deterministic ranking without requiring vectors

// Bad: Trying to be clever
// This blazingly fast algorithm will blow your mind! 🔥
```

## Community Communications

### Issues & PRs

- Thank contributors genuinely
- Explain technical decisions clearly
- Avoid excessive enthusiasm ("AMAZING PR!!!" → "Thanks for the contribution")

### Release Notes
✅ **Good**: "Fixed memory leak in search indexer, reducing memory usage by 40%"
❌ **Bad**: "MASSIVE performance improvements that make BLZ even MORE BLAZING FAST!"

## Examples

### Good Description
> BLZ indexes your llms.txt documentation locally for fast, offline search. Returns exact line numbers in 6ms typical latency. Built with Rust and Tantivy for consistent, deterministic results.

### Bad Description
> 🔥 BLZ is the ULTIMATE BLAZINGLY FAST documentation tool that REVOLUTIONIZES how you search! With INSANE speeds and CUTTING-EDGE technology, it's the BEST tool you'll EVER use! 🚀🔥💯

## Final Note

When in doubt, choose:

- Clarity over cleverness
- Accuracy over impact
- Utility over excitement

We're building tools for professionals who appreciate honest, capable software. Let the performance speak for itself.
