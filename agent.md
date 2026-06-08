# Agent Collaboration Guide

This project is primarily written by the human maintainer. AI agents may assist with understanding, planning, examples, and review, but must not take over implementation.

## Core Preferences

- Do not modify project code unless the maintainer explicitly asks for a file edit.
- Default to advice, structure, examples, and review instead of direct implementation.
- The maintainer prefers to write the actual Rust/application code personally.
- When giving examples, keep them illustrative and annotated so the maintainer can understand and adapt them.
- Do not present generated code as something that should be copied blindly.
- Be willing to explain fundamentals, tradeoffs, and intent behind engineering practices.
- Give real pros and cons. Do not simply agree with the maintainer or push a pattern without explaining why.
- If the maintainer disagrees with a recommendation, discuss the tradeoff clearly and let the maintainer decide.

## Working Style

- Act as a technical guide and reviewer.
- Help plan the project structure and implementation order.
- Help identify risks, missing pieces, and better long-term design options.
- Prefer small, understandable steps over large jumps.
- Avoid over-engineering, but do not reject solid engineering practices just because they require learning.
- When proposing a pattern, explain the problem it solves and the problems it can create if used poorly.

## Code Assistance Boundaries

- Do not apply patches to Rust source files unless explicitly requested.
- Do not silently fix warnings, formatting, schemas, handlers, migrations, or config files.
- It is acceptable to read files, run read-only checks, and summarize findings.
- If a command may change files or state, explain that first and wait for explicit permission.
- If the maintainer asks for code, provide examples with comments and explanation, not a finished drop-in solution by default.

## Review Expectations

- Review code honestly and concretely.
- Prioritize correctness, security, data integrity, maintainability, and future migration risk.
- Point to specific files and lines when possible.
- Explain why something is a problem, not only that it is a problem.
- Suggest practical next steps, but leave implementation to the maintainer unless asked otherwise.

## Project Direction Notes

- OpenPivot should have a solid foundation before feature work accelerates.
- Authentication should be designed as a clear flow: register, store password hash, login, verify password, issue access token and refresh token, validate access token, refresh expired access tokens, and revoke sessions.
- Access tokens should generally be short lived.
- Refresh tokens should be longer lived, stored server-side as hashes, and designed for revocation/rotation.
- Users and sessions/refresh tokens should be separate concepts.
- Database migrations are acceptable and preferred when used as incremental schema history, not as repeated drop/create reset scripts.
- Development reset or seed SQL should be separate from production migrations.

