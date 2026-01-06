---
allowed-tools: Bash(git status:*), Bash(git diff:*), Bash(git add:*), Bash(git commit:*), Bash(git push:*), Bash(git pull:*), Bash(git log:*)
description: Stage, commit, and push changes to remote with conflict resolution
---

# Git Workflow Command

Execute the complete git workflow:

1. **Check Status**: Run `git status` to see current changes
2. **Review Changes**: Run `git diff` to review what will be committed
3. **Stage Changes**: Add all relevant changes with `git add`
4. **Create Commit**: Generate a commit message following Vissper's style:
   - Focus on the "why" rather than the "what"
   - Keep it concise (1-2 sentences)
   - Include the standard footer:
     ```
     🤖 Generated with [Claude Code](https://claude.com/claude-code)

     Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
     ```
5. **Push to Remote**: Push the commit to the remote repository
6. **Handle Conflicts**: If push fails due to conflicts:
   - Pull latest changes
   - Resolve any merge conflicts
   - Complete the merge
   - Push again

**Important**:
- Never commit files that likely contain secrets (.env, credentials.json, etc.)
- Always use a HEREDOC for commit messages to ensure proper formatting
- If there are no changes to commit, inform the user

Execute this workflow now.
