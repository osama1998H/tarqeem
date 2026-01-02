# Create Bug Issue

Create a GitHub issue for a detected bug or problem.

## Usage

```
/project:create-bug-issue <description>
```

## Arguments

- `$ARGUMENTS` - Brief description of the bug

## Instructions

1. **Check for recent bug detection files**:
   ```bash
   ls -la /tmp/tarqeem-bug-*.json 2>/dev/null | tail -5
   ```

2. **Read the most recent bug file if it exists**:
   ```bash
   cat /tmp/tarqeem-bug-*.json 2>/dev/null | tail -1 | jq .
   ```

3. **Check for existing issues with similar description**:
   ```bash
   gh issue list --state open --search "$ARGUMENTS"
   ```

4. **If no existing issue found, gather context**:
   - Current branch: `git branch --show-current`
   - Current commit: `git rev-parse --short HEAD`
   - Recent changes: `git diff --stat HEAD~1`

5. **Create the issue** (use single-line command):
   ```bash
   gh issue create --title "[BUG] $ARGUMENTS" --label "bug" --label "auto-detected" --body "## Description
   $ARGUMENTS

   ## Environment
   - Branch: $(git branch --show-current)
   - Commit: $(git rev-parse --short HEAD)
   - Detected: $(date)

   ## Context
   This issue was created via Claude Code.

   ## Steps to Reproduce
   [Add reproduction steps]

   ## Expected Behavior
   [Add expected behavior]

   ## Actual Behavior
   [Add actual behavior]"
   ```

6. **Report the result to the user**:
   - Show the issue number and URL
   - Confirm the issue was created successfully

## Example

```
/project:create-bug-issue lexer fails on Arabic numerals
```

This will:
1. Check for existing issues about "lexer fails on Arabic numerals"
2. Create a new issue if none exists
3. Report the created issue number and URL
