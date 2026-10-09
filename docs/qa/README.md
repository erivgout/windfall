# QA evidence

The 2026-10-09 pass follows the user's instruction to verify implementation
already present, repair failures, and mark accepted features done. The earlier
implementation-first QA deferral is superseded. All checks run locally;
GitHub CI and Actions are not used.

The final pass accepted 46 new completions. Current accounting is 136 done,
104 in progress, 100 todo and two exclusions. The [completion report](2026-10-09-completion.md)
lists exact accepted IDs, successful final gates and the remaining scope.

`../parity/parity.json` remains the completion source of truth. Its `done`
status means the full row's software behavior has accepted evidence. A passing
test of a subset does not complete an unfinished integration, device workflow,
plugin capability, or platform promise. Reports distinguish runtime tests,
source review, and outstanding manual acceptance.

This pass tests the dirty working tree based on
`be782b3c86058006c96c3374cc86f757b88c91a2`; that commit alone does not contain
the implementation or repairs under test. The execution reports record commands,
results, and generated artifact checks for the combined tree.

- [Completion acceptance and final gates](2026-10-09-completion.md)
- [Implementation inventory](2026-10-09-inventory.md)
- [Whole-feature completion audit](2026-10-09-completion-audit.md)
- [Frontend checks](2026-10-09-frontend.md)
- [Native checks and artifacts](2026-10-09-native.md)
- [Piano gestures](2026-10-09-rack-piano.md)
- [Piano randomizer and timeline contracts](2026-10-09-piano-contracts.md)
- [Recording and mixer](2026-10-09-recording-mixer.md)
- [Playlist and bounce](2026-10-09-playlist-workflow.md)
- [Arrangement transactions](2026-10-09-arrangement-repair.md)
- [DSP, plugins, and analyzers](2026-10-09-dsp-plugins.md)
- [Delay-bank integration and remaining gates](2026-10-09-delay-bank.md)
- [Plain notebook acceptance](2026-10-09-notebook.md)
- [Analysis UI and IPC](2026-10-09-analysis-repair.md)
- [Workflow and channel groups](2026-10-09-workflow.md)
- [Channel timing acceptance](2026-10-09-channel-timing.md)
- [Mixer UI and scale acceptance](2026-10-09-mixer-ui.md)
- [Engine warning cleanup](2026-10-09-engine-cleanup.md)
- [Project fixture reconciliation](2026-10-09-project-fixture-reconciliation.md)
- [DSP runtime harness reconciliation](2026-10-09-dsp-runtime-harness.md)
- [Live browser smoke check](2026-10-09-browser.md)

Logs retain initial failed attempts as diagnostic evidence. Use each report's
final accepted command results, rather than treating every historical log as
the final outcome.

The tracker reconciliation moved 85 ledger-mapped rows from `todo` to
`in-progress`: their documented implementation is a subset awaiting full-row
acceptance. This is not a QA pass or a completion promotion. Together with the
eight earlier scope corrections, it removes stale accounting while retaining
the actual remaining gates. Final accepted promotions are recorded separately.
