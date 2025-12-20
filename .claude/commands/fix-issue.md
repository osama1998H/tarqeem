# Fix Issue

Fix the issue: $ARGUMENTS

## Instructions

Follow the full safe-change workflow to fix this issue.

### Phase 1: UNDERSTAND

1. **Reproduce the issue** - What exact input triggers it?
2. **Locate the bug** - Which file/function contains the bug?
3. **Understand the root cause** - Why does this happen?
4. **Check for related issues** - Are there similar bugs elsewhere?

### Phase 2: EXPLORE

1. **Find the relevant code** - Grep/read the affected files
2. **Understand the context** - What is this code supposed to do?
3. **Check tests** - Are there tests that should catch this? Why didn't they?
4. **Find similar fixes** - Has a similar bug been fixed before?

### Phase 3: PLAN

1. **Describe the fix** - What change will solve this?
2. **Minimal diff** - What's the smallest change that fixes it?
3. **Regression test** - What test will prove the fix works?
4. **Side effects** - Could this fix break anything else?

### Phase 4: IMPLEMENT

1. **Apply the fix** - Make the minimal change
2. **Add regression test** - Write a test that fails without the fix
3. **Check related code** - Fix similar issues if found

### Phase 5: VERIFY

Run:
```bash
cargo fmt
cargo clippy
cargo test
```

The fix is NOT complete until:
- [ ] All tests pass
- [ ] New regression test exists
- [ ] No new warnings from clippy

### Phase 6: DOCUMENT

Update `docs/AI_NOTES.md` with:
- Issue description
- Root cause
- Fix applied
- Test added
