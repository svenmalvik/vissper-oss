# Documentation Standards for Markdown Files

## Scope
This rule applies to all `*.md` files within the `/docs` folder.

## General Principles
- Documentation is a first-class deliverable, not an afterthought
- Write for clarity, not cleverness
- Keep docs synchronized with code changes
- Use examples to illustrate concepts
- Assume readers have varying levels of technical expertise

## Structure and Organization

### File Naming
- Use lowercase with hyphens: `phase-3-setup.md`
- Be descriptive: prefer `stripe-integration-guide.md` over `guide.md`
- Group related docs in subdirectories when appropriate

### Document Structure
1. **Title**: Single H1 (`#`) at the top
2. **Overview**: Brief description of what this document covers
3. **Table of Contents**: For documents longer than 3 sections
4. **Main Content**: Logical sections with H2-H4 headings
5. **References/Links**: Related documentation at the end

```markdown
# Feature Name or Guide Title

Brief overview of what this document covers (1-2 sentences).

## Table of Contents
- [Section 1](#section-1)
- [Section 2](#section-2)

## Section 1
Content here...

## References
- [Related Doc 1](./related-doc.md)
```

## Formatting Standards

### Headings
- Use ATX-style headings (`#`, `##`, `###`)
- One H1 per document (the title)
- Don't skip heading levels (H1 → H2 → H3, not H1 → H3)
- Add blank lines before and after headings

### Code Blocks
- Always specify language for syntax highlighting
- Use inline code (backticks) for commands, file names, variables
- Add comments to complex code examples
- Include expected output where helpful

```typescript
// Good: language specified with descriptive comment
const user = await supabaseClient.auth.getUser();
```

### Lists
- Use `-` for unordered lists (not `*` or `+`)
- Use `1.` for ordered lists (auto-numbering)
- Indent nested lists with 2 spaces
- Add blank lines between list items if they contain multiple paragraphs

### Links
- Use descriptive link text: `[deployment guide](./deployment.md)`, not `[click here](./deployment.md)`
- Use relative paths for internal docs: `./setup.md` or `../api/endpoints.md`
- Verify links work (no broken references)

### Emphasis
- Use `**bold**` for important terms or UI elements
- Use `*italic*` for emphasis or introducing new concepts
- Use `code` for technical terms, file paths, commands

## Content Guidelines

### Code Examples
- Provide complete, runnable examples when possible
- Show both success and error cases where relevant
- Include setup/prerequisites if needed
- Explain what the code does, not just what it is

### Procedural Documentation
- Number sequential steps clearly
- Include expected outcomes: "You should see..."
- Mention common pitfalls or gotchas
- Provide troubleshooting tips

Example:
```markdown
## Setting Up Authentication

1. Create a new Supabase project at [supabase.com](https://supabase.com)
2. Navigate to **Authentication** → **Providers** in the dashboard
3. Enable Google OAuth provider
4. Copy the Client ID and Secret

**Expected result**: You should see "Google" listed as an active provider.

**Troubleshooting**: If you see "Invalid redirect URI", check that...
```

### Technical Accuracy
- Test all commands and code examples before documenting
- Keep version numbers up-to-date
- Note platform-specific differences (macOS vs Windows)
- Update docs when implementation changes

### Accessibility
- Use descriptive alt text for images: `![Supabase dashboard showing auth settings](./images/auth-dashboard.png)`
- Don't rely solely on color to convey information
- Ensure tables are simple and have headers

## Special Sections

### Status Badges (Optional)
For guides or feature docs, consider adding status:
```markdown
**Status**: ✅ Production | 🚧 In Progress | ⚠️ Deprecated
**Last Updated**: 2025-12-07
```

### Prerequisites
List required knowledge, tools, or setup steps:
```markdown
## Prerequisites
- Node.js 18+ installed
- Supabase CLI configured
- Basic understanding of TypeScript
```

### Related Documentation
Always link to related docs at the end:
```markdown
## See Also
- [Phase 2 Setup](./phase-2-setup.md)
- [Troubleshooting Guide](./troubleshooting.md)
- [API Reference](../api/README.md)
```

## Maintenance

### When to Update Docs
- When adding new features
- When changing existing functionality
- When fixing bugs that affect documented behavior
- When deprecating features
- When receiving questions that documentation should answer

### Review Checklist
Before committing documentation changes:
- [ ] All code examples tested and working
- [ ] Links are valid (no 404s)
- [ ] Spelling and grammar checked
- [ ] Consistent formatting throughout
- [ ] Table of contents updated (if present)
- [ ] Screenshots current (if present)

## Examples

### Good Documentation
```markdown
# Stripe Webhook Integration

This guide explains how to set up and test Stripe webhooks for handling subscription events in Vissper.

## Prerequisites
- Stripe account with API keys
- Supabase project deployed
- Stripe CLI installed for local testing

## Setup Steps

1. **Create webhook endpoint in Stripe**
   
   Navigate to Stripe Dashboard → Developers → Webhooks and add a new endpoint:
   
   ```
   https://your-project.supabase.co/functions/v1/stripe-webhook
   ```

2. **Configure webhook events**
   
   Select the following events:
   - `checkout.session.completed`
   - `customer.subscription.updated`
   - `customer.subscription.deleted`

3. **Add webhook secret to Supabase**
   
   Copy the webhook signing secret and add it to your Supabase project:
   
   ```bash
   supabase secrets set STRIPE_WEBHOOK_SECRET=whsec_...
   ```

## Testing Locally

Use the Stripe CLI to forward events to your local development server:

```bash
stripe listen --forward-to localhost:54321/functions/v1/stripe-webhook
```

Trigger a test event:

```bash
stripe trigger checkout.session.completed
```

**Expected output**: You should see the webhook processed in your terminal logs.

## Troubleshooting

### Error: "Webhook signature verification failed"
This usually means the webhook secret doesn't match. Verify:
- The secret is correctly set in Supabase: `supabase secrets list`
- The secret matches your Stripe dashboard

## See Also
- [Subscription Management](./subscription-management.md)
- [Stripe API Reference](https://stripe.com/docs/api)
```

## Anti-Patterns to Avoid

❌ **Don't**: Use vague language
```markdown
You might want to configure some settings here.
```

✅ **Do**: Be specific
```markdown
Set the `timeout` value to 30 seconds in `config.json`.
```

❌ **Don't**: Assume too much knowledge
```markdown
Just set up RLS policies as usual.
```

✅ **Do**: Provide guidance or links
```markdown
Configure Row Level Security (RLS) policies. See [RLS Setup Guide](./rls-setup.md) for details.
```

❌ **Don't**: Leave examples incomplete
```typescript
const result = await someFunction(
```

✅ **Do**: Show complete, working code
```typescript
const result = await someFunction(userId, options);
console.log(result);
```
