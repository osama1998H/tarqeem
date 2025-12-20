# Add Feature

Add the feature: $ARGUMENTS

## Instructions

Follow the full safe-change workflow for new features.

### Phase 1: REQUIREMENTS

1. **Clarify the feature** - What exactly should this do?
2. **Define scope** - What's in scope? What's out of scope?
3. **Identify edge cases** - What unusual inputs/states must be handled?
4. **Check existing features** - Does something similar already exist?

### Phase 2: EXPLORE

1. **Find related code** - Where do similar features live?
2. **Understand patterns** - How are similar features implemented?
3. **Check the pipeline** - Which compiler phases need changes?
4. **Find test patterns** - How are similar features tested?

### Phase 3: DESIGN

1. **Architecture** - How does this fit into the compiler?
2. **API** - What's the public interface?
3. **Data structures** - What new types are needed?
4. **Pipeline integration** - How does data flow through?

### Phase 4: PLAN

1. **Implementation steps** - Ordered list of what to do
2. **Files to modify/create** - Exact paths
3. **Tests to write** - What tests prove correctness?
4. **Bilingual support** - Arabic + English for user-facing parts

### Phase 5: IMPLEMENT

For each compiler phase affected:

1. **Lexer** - New tokens?
2. **Parser** - New AST nodes?
3. **Semantic** - New type rules?
4. **IR** - New IR instructions?
5. **Codegen** - New code generation?
6. **Tests** - Tests for each phase

### Phase 6: VERIFY

```bash
cargo fmt
cargo clippy
cargo test
```

Feature is NOT complete until:
- [ ] All phases are implemented
- [ ] Tests exist for each phase
- [ ] Arabic + English supported
- [ ] All tests pass

### Phase 7: DOCUMENT

Update:
- `docs/AI_NOTES.md` - Implementation notes
- `README.md` - If user-visible
- `CLAUDE.md` - If new patterns established
