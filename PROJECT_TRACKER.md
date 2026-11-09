# Project Tracker — GitHub Projects

## Setup

- **Project:** "Evo Intel" (ID `1`)
- **Owner:** `formasimplex` (org)
- **Repo:** `formasimplex/evointel`
- **CLI scope required:** `gh auth refresh -s project` if commands fail with permission errors.

## Reading

```bash
# List all items (title, status, id)
gh project item-list 1 --owner formasimplex

# List with JSON (for filtering by status, labels, etc.)
gh project item-list 1 --owner formasimplex --format json

# View the project in browser (for humans)
gh project view 1 --owner formasimplex --web
```

## Creating items

```bash
# Draft item (no repo issue)
gh project item-create 1 --owner formasimplex --title "Short title" --body "Description"

# Real repo issue (appears in repo + project)
gh issue create --repo formasimplex/evointel --title "Short title" --body "Description"
gh project item-add 1 --owner formasimplex --item <ISSUE_URL_OR_NUMBER>
```

Prefer **real issues** for anything that will have a PR. Use draft items for quick notes or planning items.

## Updating items

```bash
# Get item ID (needed for edits)
gh project item-list 1 --owner formasimplex --format json -q '.[] | select(.content.title | test("your-title")) | .id'

# Change status (Todo / In Progress / Done)
gh project item-edit <ITEM_ID> --owner formasimplex --status "In Progress"

# Change title
gh project item-edit <ITEM_ID> --owner formasimplex --title "New title"

# Add body/description
gh project item-edit <ITEM_ID> --owner formasimplex --body "Updated description"

# Set assignee
gh project item-edit <ITEM_ID> --owner formasimplex --assignees WSINTRA

# Set labels
gh project item-edit <ITEM_ID> --owner formasimplex --labels "security,phase-1"

# Set milestone
gh project item-edit <ITEM_ID> --owner formasimplex --milestone "Phase 1 — Security"

# Archive (soft-delete)
gh project item-archive <ITEM_ID> --owner formasimplex
```

## Linking PRs

```bash
# Add a PR to the item
gh project item-edit <ITEM_ID> --owner formasimplex --add-link <PR_URL>

# When PR merges, mark item Done
gh project item-edit <ITEM_ID> --owner formasimplex --status "Done"
```

## Conventions

- **Start work:** set status to `In Progress` before making changes.
- **Finish work:** set status to `Done` when the PR merges (or when work is complete for draft items).
- **Blocked:** add label `blocked` and note the blocker in the item body.
- **Naming:** item titles should be imperative and specific — "Implement WebCrypto KDF for AES-GCM" not "encryption stuff".
- **One item per unit of work** that produces a PR. Broader scope gets split into sub-items.
