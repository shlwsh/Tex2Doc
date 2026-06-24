# Commercialization Maturity Rubric

## Maturity Levels

| Level | Use When | Commercialization Advice |
|---|---|---|
| 初级阶段 | Core product is still validating feasibility, major workflows are incomplete, or quality/reliability is unproven. | Do not sell broadly. Continue product validation and internal testing. |
| 可试点阶段 | Core value can be demonstrated and early users can succeed with support, but delivery, operations, billing, quality proof, or onboarding are not yet repeatable. | Run controlled PoC or invite-only beta with hands-on support. |
| 可商业化阶段 | Product has stable core workflows, repeatable onboarding/deployment, basic billing/support, clear pricing, and production safeguards. | Begin formal promotion to a focused ICP and convert early paid customers. |
| 可规模化阶段 | Product has standardized deployment, self-serve acquisition, automated upgrades, monitoring/SLA, customer success motions, and proven retention. | Scale go-to-market channels and partner/channel programs. |

## Dimension Questions

### 易用性 Usability

- Can target users understand the product and complete the first valuable task without engineering help?
- Are core task paths short, visible, and named in the user's vocabulary?
- Are error states actionable and tied to recovery steps?
- Are examples, templates, demo data, onboarding, and help docs available?

### 可用性 Availability / Stability

- Are core workflows covered by automated and manual regression checks?
- Are data, uploads, jobs, usage, billing, feedback, and conversion records durable after restart?
- Are errors observable with stable error codes, logs, diagnostics, and reports?
- Are performance, concurrency, queueing, retry, timeout, sandbox, monitoring, and alerting designed for real users?

### 可获得性 Accessibility / Reachability

- Can customers discover the product through a website, docs, marketplace, community, SEO, or partner channels?
- Is there a clear trial/download/registration path and a demo that sales can show?
- Are pricing, plan boundaries, FAQ, comparison material, and purchase path clear?
- Can leads be captured and tracked through a lightweight CRM or waitlist?

### 易部署性 Deployability

- Is deployment standardized for SaaS, desktop, server, private deployment, and customer environments?
- Are installers, signed packages, Docker/systemd/Kubernetes docs, environment checks, and configuration templates available?
- Can support staff reproduce customer setups and collect diagnostics?
- Are initial accounts, permissions, sample data, plans, quotas, and storage paths initialized safely?

### 易升级性 Upgradability

- Are version checks, release manifests, release notes, signatures, rollback, and gray rollout supported?
- Are data/schema migrations documented and tested?
- Can customer-specific versions or channels be managed?
- Can users upgrade without losing local settings, tokens, usage records, or jobs?

## Priority Rules

P0 items directly block customer trust, first successful use, delivery, data safety, payment, or support.

P1 items reduce conversion, activation, retention, repeatable delivery, support efficiency, or paid beta readiness.

P2 items improve differentiation, scale, growth loops, brand authority, and long-term competitive advantage.

## Report Style

- Prefer concrete actions over slogans.
- Tie every major issue to commercial impact.
- Name the product artifact or workflow that should change.
- Include missing evidence as a gap.
- Avoid claiming readiness without proof from docs, tests, operational design, or working product paths.
