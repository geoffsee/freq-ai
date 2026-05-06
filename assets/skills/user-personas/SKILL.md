---
name: user-personas
description: Use when doing UXR synthesis or interpreting research signal about people who adopt, deploy, operate, or build on top of the project.
---

# User Personas

Use this skill when you have research signal — interview snippets, support tickets, telemetry, issue reports, or other research signal — and need to apply a persona-based lens to your analysis.

## How to use during synthesis

1. **Tag.** For each piece of evidence, identify the persona whose `recognition_cues:` it most directly matches.
2. **Weight.** Interpret the evidence against that persona's specific goals, pains, and constraints.
3. **Flag blind spots.** If signal does not match any persona, mark it as a potential persona blind spot.

## Persona files

Personas are stored as JSON documents in the `personas/` subdirectory.
The desktop UI's Personas Studio reads and writes the same directory. If
`freq-ai.toml` overrides `[skills].user_personas`, the studio stores persona JSON
beside that custom `SKILL.md`; otherwise it uses this bundled skill's
`personas/` directory.

## Schema and required fields

Each persona file should include:

- `jobs_to_be_done:` — top jobs this project could help them accomplish.
- `pains:` — current frustrations that create switching energy.
- `adoption_yes_if:` — capabilities or signals that make the project a credible yes.
- `rejection_no_if:` — capabilities or gaps that make the project a no.
- `anti_goals:` — what they explicitly do not care about.
- `recognition_cues:` — phrases, requests, objections, or behaviors that let you tag evidence to this persona.

The bundled personas use TinyPerson-style JSON:

```json
{
  "type": "TinyPerson",
  "persona": {
    "name": "Persona Name",
    "occupation": {
      "title": "Role",
      "organization": "Organization",
      "description": "Short context summary"
    },
    "communication_style": "How this user tends to communicate",
    "other_facts": [
      "jobs_to_be_done: ...",
      "pains: ...",
      "adoption_yes_if: ...",
      "rejection_no_if: ...",
      "anti_goals: ...",
      "recognition_cues: ..."
    ]
  }
}
```

## What this skill is not

- Not marketing copy or TAM sizing.
- Not a roadmap or feature prioritization.
- Not personas of contributors or maintainers of the project itself.
