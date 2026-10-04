# Moonships Platform Roadmap

The active roadmap is now the focused v3 production plan:

- [Moonships v3 Production Grand Plan](grand-plan-v3.md)
- Root [ROADMAP.md](../ROADMAP.md)
- [Branch Strategy](branch-strategy.md)

GitHub issue #90 is the umbrella plan.

Release sequence:

```
v1.0 GA
  -> v1.1 deploy UX
  -> v1.2 stateful services
  -> v1.3 workers/jobs
  -> v2.0 desired state + promotion
  -> v2.1 multi-server placement/recovery
  -> v2.2 drift + bounded auto-recovery
  -> v3.0 production platform GA
```

The roadmap intentionally excludes Kubernetes, marketplace, billing, enterprise federation, serverless, global edge scheduling, and provider-side VPS provisioning until after v3.

Production-readiness gates must be reproducible without a real VPS by using local/CI Docker, test SSH targets, fixtures, failure injection, and published-image smoke tests.
