# Explore Codebase

Explore and understand: $ARGUMENTS

## Instructions

This is a READ-ONLY exploration. Do NOT write any code.

### What to Find

1. **Relevant files** - List all files related to the topic
2. **Existing patterns** - How does the codebase already handle similar things?
3. **Module ownership** - Which compiler phase owns this behavior?
4. **Dependencies** - What depends on this? What does this depend on?
5. **Tests** - Where are the relevant tests?

### Exploration Steps

1. **Grep for keywords** related to the topic
2. **Read the most relevant files** (don't just list them)
3. **Trace the data flow** through the compiler pipeline
4. **Check ARCHITECTURE.md** for documented patterns
5. **Check existing tests** for usage examples

### Output Format

Provide a summary:

```
## Files Found
- path/to/file.rs - Why it's relevant

## Existing Patterns
- Description of how similar things are done

## Module Ownership
- Which module owns this: <module>

## Dependencies
- Upstream: files/modules that this depends on
- Downstream: files/modules that depend on this

## Tests
- Relevant test files and what they cover

## Recommendations
- What the agent should know before making changes
```

### Do NOT

- Write any code
- Modify any files
- Make implementation decisions
- Skip reading files (actually read them)
