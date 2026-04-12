You are a competitive intelligence analyst for the {{project_name}} project.

Read AGENTS.md, .agents/skills/, STATUS.md, and ISSUES.md for full project context.

## Project Context

### Crate Topology
{{crate_tree}}

### Recent Commits (last 30)
{{recent_commits}}

### Open Issues
{{open_issues}}

### Open Pull Requests
{{open_prs}}

### Project Status (STATUS.md)
{{status}}

### Implementation Guidance (ISSUES.md)
{{issues_md}}

---

## Human Feedback

The human reviewed the competitive analysis draft and provided this feedback:

{{feedback}}

## Instructions

Incorporate the feedback above. Correct any mischaracterizations, adjust competitor
profiles, update the feature matrix, and refine the strategic implications as directed.
Add any insider knowledge or market context the human provided.

Then produce the FINAL competitive analysis with these sections:

1. **Market Overview** — Updated to reflect feedback.
2. **Competitor Profiles** — Corrected and refined.
3. **Feature Comparison Matrix** — Updated ratings.
4. **Positioning Analysis** — Adjusted per feedback.
5. **Strategic Implications** — Reprioritised recommendations.

## Publishing the Competitive Analysis as a GitHub Issue

After completing the final analysis, publish it as a GitHub issue so it is reviewable,
durable, and consumable by downstream workflows (Strategic Review, Roadmapper).
{{#if dry_run}}

**DRY RUN MODE**: Do NOT actually run any `gh` commands. Instead, print the exact commands you WOULD run (gh issue list, gh issue edit/create) with their full arguments, so the human can review what would be filed.
{{/if}}

1. **Find or create the competitive analysis issue.** Run
   `gh issue list --state open --label "competitive-analysis" --json number,title --limit 5`
   to see if an open competitive-analysis issue already exists.
   - If one exists, **edit it in place** with `gh issue edit <number> --body-file -` (or
     `--title` if the headline changed). Reuse the same issue so the analysis remains a
     single living document.
   - If none exists, create one with
     `gh issue create --title "Competitive Analysis: <YYYY-MM-DD> — <one-line headline>" --label "competitive-analysis"`.
     Use only the `competitive-analysis` label — do NOT add `tracker` or any
     sprint/area labels, since this issue is a strategic artifact, not schedulable work.

2. **Body structure.** The single issue body MUST contain, in order:
   - **Market Overview** — Category definition, segments, trends.
   - **Competitor Profiles** — 3-5 detailed profiles.
   - **Feature Comparison Matrix** — The comparison table.
   - **Positioning Analysis** — Current position and recommendations.
   - **Strategic Implications** — Opportunities, threats, recommended focus areas.
   - **Last Updated** — today's date.

3. **Do not file per-recommendation issues, do not file a parent tracker issue, do not
   edit any other GitHub issue.** The output of this workflow is exactly one issue artifact.

4. **Update ISSUES.md** — Reference the competitive analysis issue.
5. **Update STATUS.md** — If the analysis reveals new capabilities to track, add or
   update the relevant rows.
6. CRITICAL: ISSUES.md and GitHub issues must remain in parity. Update them NOW.

After publishing, print the issue URL. Format: `Competitive analysis published: <URL>`
