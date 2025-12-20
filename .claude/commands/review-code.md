# Review Code

Review the code: $ARGUMENTS

## Instructions

Perform a thorough code review focusing on Tarqeem-specific concerns.

### Review Checklist

#### 1. Architecture Compliance

- [ ] Does the code respect layer boundaries? (See `.claude/rules/architecture.md`)
- [ ] Is code in the correct module?
- [ ] Are dependencies going in the right direction?

#### 2. Error Handling

- [ ] No `unwrap()` or `expect()` on user input
- [ ] Errors are recoverable where possible
- [ ] Error messages have both English AND Arabic

#### 3. Arabic Support

- [ ] New keywords have Arabic primary + English alias
- [ ] User-facing strings are bilingual
- [ ] Unicode/NFC normalization is correct
- [ ] Arabic punctuation is accepted

#### 4. Code Quality

- [ ] Follows Rust idioms (See `.claude/rules/rust-style.md`)
- [ ] No unnecessary cloning
- [ ] Types are used correctly
- [ ] No dead code

#### 5. Testing

- [ ] Tests exist for new functionality
- [ ] Tests cover both Arabic and English
- [ ] Edge cases are tested
- [ ] Error paths are tested

#### 6. Performance

- [ ] No obvious inefficiencies
- [ ] Allocations are reasonable
- [ ] No accidental O(n²) loops

### Output Format

```
## Summary
<1-2 sentence summary of review>

## Issues Found

### Critical (must fix)
- [ ] Issue description | file:line

### Important (should fix)
- [ ] Issue description | file:line

### Minor (nice to fix)
- [ ] Issue description | file:line

## Positive Observations
- What was done well

## Recommendations
- Suggested improvements
```

### Verify After Fixes

After issues are addressed, run:
```bash
cargo fmt --check
cargo clippy
cargo test
```
