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

## Schema and required fields

Each persona file should include:

- `jobs_to_be_done:` — top jobs this project could help them accomplish.
- `pains:` — current frustrations that create switching energy.
- `adoption_yes_if:` — capabilities or signals that make the project a credible yes.
- `rejection_no_if:` — capabilities or gaps that make the project a no.
- `anti_goals:` — what they explicitly do not care about.
- `recognition_cues:` — phrases, requests, objections, or behaviors that let you tag evidence to this persona.

## What this skill is not

- Not marketing copy or TAM sizing.
- Not a roadmap or feature prioritization.
- Not personas of contributors or maintainers of the project itself.
