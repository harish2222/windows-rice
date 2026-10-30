# PR Gate Result — run ed156b73 (multi-agent roster, 26 agents, manual-takeover completion)

**Verdict: FAIL** (detection-only; no files modified) · **Date:** 2026-10-30 · **Review:** `ed156b73-7fb1-41a2-a219-890a61b1554a` · **Agent run:** `5f03e566b8ea8a032e349caac20ec88d` (26/26 terminal)

## Outcome summary

- Roster completed manually after the supervisor orphaned itself at 69% (18/26): all 11 unfinished
  lanes (3 failed in-band + 8 never scheduled) were completed by operator takeover with full
  file-read access; per-agent findings JSONs at `.mcp/agent-runs/<runId>/<agent>.json`; attestation
  chain grew from 11 to **25 links, valid=true** (`security_verify_chain`, head recorded in
  `attestation-chain.json`).
- `orchestration_assert_run_complete`: complete=true, 26/26 terminal, blocker=null.
- Skill coverage: **64%** (WARN — 10 sections uncovered: §1, §3, §7, §9, §11, §12, §14, §15, §20, §24;
  NA lanes cannot honestly claim them).
- `orchestration_merge_agent_findings`: merged 28 findings across 26 agents (server-bucketed:
  1 critical / 11 high / 16 medium / 0 low); gate for run ed156b73 recorded **status FAIL**
  (drivers below); `run_pr_gate` step auto-populated from the merge.
- `security_attest_review` **refused** to attest a FAIL gate (correct doctrine — resolve or
  risk-accept findings first; forcing via SECURITY_ATTEST_ALLOW_INCOMPLETE was deliberately not used).
- SOC2 compliance report generated and persisted: `security_generate_compliance_report(SOC2)`
  → 9 controls: 8 satisfied, 0 missing, 1 unverified (self_heal_loop — remediation awaiting
  operator approval by design).

## Verdict drivers (gate FAIL)

1. **Substantive:** new CRITICAL verified by manual takeover —
   `APS-001`: a live 64-char VirusTotal API key inside the tracked, committed, pushed
   `scoop/config.json` (committed blob has `virustotal_api_key`; length-presence verified, value
   never emitted). This finding alone blocks attestation until revoked/rotated + untracked.
2. **Substantive (residual):** two MEDIUM advisories remain open without operator decision —
   `EXEC-POLICY-WIDEN` (install.ps1:430) and `EXT-TOOL-REMOTE-EVAL-WINUTIL` (xonsh rc.xsh:399/402).
3. **Process:** skill coverage 64% < required 90% (honest NA lanes).
4. **Process:** `CAPABILITY_FLOOR_NOT_MET` — manual lanes flagged by the floor check (adapter
   class "manual"); the 10 in-band codex agents were ALSO flagged, so the floor reflects degraded
   provider capability in-band as much as the takeover.
5. **Tooling caveat (not a finding):** `signatureVerification.rejectedAgents` lists 11
   `payload-mismatch` rejections — 10 of them are the in-band completed agents whose files were
   authored by the pre-crash supervisor (its attest hashes vs. normalized files disagree);
   `evidence-collector` (zero findings, manual) also mismatched. Chain itself verifies valid=true.
6. **Tooling caveat:** stale `exhausted retries → escalation` note for the 3 originally-failed
   lanes is server-side retry accounting from the orphaned supervisor; the manifest counters were
   corrected to 0 but the in-memory tally persisted.

## Takeover agent map (11 lanes)

| Lane | Terminal state | Key results |
|---|---|---|
| appsec-code-auditor | completed | APS-001 CRITICAL (VT key, tracked repo file), APS-002 LOW (TOCTOU), HIGH-triplet fix-verified |
| serialization-memory-attacker | completed | SER-001 LOW informational (typed serde, fail-open; zero production unsafe) |
| key-management-lifecycle-analyst | completed | KM-001 LOW (checksum-only release integrity), KM-002 LOW (no rotation/vault/secret-scan doctrine) |
| cloud-infra-specialist | completed | CLD-001 LOW (ci.yml default token scopes), CLD-002 LOW remediated (v1.0.4 hardening verified) |
| pentest-web-api | completed_na | no HTTP/listener surface (sweeps verified) |
| pentest-infra | completed_na | no services/tasks/firewall vectors; CI surface covered by cicd-pipeline-hijacker |
| pentest-team | completed_na | no messaging/webhook surface (glyph-only matches) |
| pentest-social | completed | copy-paste lure vector verified MITIGATED by v1.0.4 (PSOC-001 remediated) |
| compliance-grc | completed | GRC-001 LOW (ACCEPT tickets absent while policy requires them) |
| evidence-collector | completed | evidence chain verified end-to-end; 0 findings |
| compliance-gap-analyst | completed | GAP-001 LOW remediated (3 HIGHs closed), GAP-002 LOW open (2 MEDIUMs in limbo) |

Full datasets: `.mcp/agent-runs/5f03e566b8ea8a032e349caac20ec88d/merged-findings.json`,
`attestation-chain.json` (25 links, valid=true). Compliance: `.mcp/reports/ed156b73-…compliance-soc2.{json,md}`.
