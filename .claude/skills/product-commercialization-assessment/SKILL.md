---
name: product-commercialization-assessment
description: Use when Codex needs to evaluate whether a software product, SaaS, B2B platform, AI tool, private-deployment solution, or internal tool is ready for commercialization, pilot rollout, paid beta, or scalable go-to-market. Produces maturity conclusions, usability/availability/accessibility/deployability/upgradability assessments, prioritized improvements, business model recommendations, and 30/60/90 day action plans.
---

# Product Commercialization Assessment

## Workflow

1. Collect product facts from README, docs, release notes, design plans, source structure, tests, deployment docs, and available product screenshots or demos.
2. State assumptions explicitly when market, pricing, customer feedback, production telemetry, or sales data is missing.
3. Evaluate the product through six lenses:
   - Product maturity and scope clarity.
   - Usability: onboarding, first task success, information architecture, user guidance, error recovery.
   - Availability and stability: reliability, performance, observability, data durability, security boundaries.
   - Accessibility and reachability: discovery, acquisition, registration, trial, pricing, demos, sales enablement.
   - Deployability: installation, environment checks, automation, private deployment, tenant/customer setup.
   - Upgradability: versioning, release notes, migration, rollback, compatibility, customer-version management.
4. Separate "commercialization blocker" from "product improvement":
   - P0: blocks customer use, delivery, trust, payment, or support.
   - P1: hurts conversion, retention, repeatable delivery, or support efficiency.
   - P2: improves differentiation, growth efficiency, brand trust, or long-term defensibility.
5. Recommend a commercialization path that matches the current evidence. Prefer controlled Preview/PoC/Beta when production operations, quality proof, onboarding, billing, or compliance are incomplete.
6. Write the report in Chinese when the project or user request is Chinese. Make it useful to product, engineering, operations, sales, and leadership readers.

## Evidence Checklist

Look for:

- Product positioning, ICP, target users, major use cases, and current lifecycle stage.
- Core workflow docs, user guides, onboarding material, screenshots, demos, and sample data.
- Test results, E2E scripts, CI, quality gates, benchmark/sample corpus, openability checks, and known limitations.
- Deployment docs, installers, Docker/Kubernetes/systemd examples, environment variables, and upgrade strategy.
- Auth, billing, usage ledger, storage persistence, queueing, monitoring, alerting, support, and diagnostics.
- Marketing assets: landing page, pricing, trial path, waitlist, demo package, comparison material, FAQ, SLA, legal/security docs.

If evidence is absent, mark it as a gap instead of assuming it exists.

## Output Structure

Use this structure unless the user asks for a different format:

1. Overall commercialization maturity conclusion:
   - Level: 初级阶段 / 可试点阶段 / 可商业化阶段 / 可规模化阶段.
   - Evidence and reasoning.
   - Core conclusion.
   - Top 3 blockers.
2. Detailed evaluation table:
   - 易用性, 可用性, 可获得性, 易部署性, 易升级性.
   - Current maturity, main problems, impact, concrete improvement suggestions.
3. Priority roadmap:
   - P0, P1, P2 with issue, reason, action, expected benefit.
4. Commercialization recommendations:
   - Business model.
   - Target market entry.
   - Sales and growth strategy.
   - Required sales/support/legal/technical materials.
5. 30/60/90 day plan:
   - Core goal, key actions, expected results.
6. Evidence appendix:
   - List the project files, docs, commands, or code areas used as basis.

## Reference

Read `references/maturity-rubric.md` when producing a full report or when the user asks for a formal maturity assessment.
