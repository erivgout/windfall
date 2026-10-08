# Toolbar memory readout

The transport toolbar now shows RAM beside the existing engine CPU load,
dropout counter and master output meter. RAM means resident memory of the
native desktop host, in binary MiB or GiB. It excludes separately hosted
webviews and plugin helper processes. The hover hint, tooltip and accessible
label state that scope. This is OS process accounting, not a sample-cache
budget, heap counter or total system memory reading.

`process_memory` runs on Tauri's blocking worker without Session, document,
controller or audio locks. Its scalar response is resident bytes or null.
The frontend queries immediately, then waits one second after each completed
response before requesting another. A slow request cannot accumulate polls.
Failed queries clear the old value and retry; unmount and Strict Mode cleanup
discard late responses and stop their timers. Browser preview returns null
and displays an unavailable dash rather than invented memory usage.

Windows reads current `WorkingSetSize` with `K32GetProcessMemoryInfo` and the
process pseudo-handle. Linux reads at most 257 bytes from `/proc/self/statm`,
validates its seven page counts and multiplies resident pages by the native
page size with checked arithmetic. The Linux kernel's scalable counts are
approximate. macOS uses `task_info(MACH_TASK_BASIC_INFO)` and the existing
pinned libc ABI's resident byte count. Unsupported platforms and failed or
incomplete native queries return null. Both sides enforce the JavaScript
safe-integer domain; negative, fractional and nonfinite UI values are refused.

The desktop crate adds references only to already locked `libc 0.2.190` and
`windows-sys 0.61.2`. Cargo.lock adds those two references to the desktop
dependency list; all existing package versions/checksums remain unchanged.
Both packages are MIT OR Apache-2.0. No Session/realtime DTO, IPC generated
type, DSP, model, controller or WASM source input changed.

Primary platform contracts:

- [Microsoft process memory accounting](https://learn.microsoft.com/en-us/windows/win32/psapi/process-memory-usage-information),
  [query API](https://learn.microsoft.com/en-us/windows/win32/api/psapi/nf-psapi-getprocessmemoryinfo)
  and [counter fields](https://learn.microsoft.com/en-us/windows/win32/api/psapi/ns-psapi-process_memory_counters).
- [Linux procfs statm documentation](https://www.kernel.org/doc/html/v6.16/filesystems/proc.html).
- [Apple Mach task-info ABI](https://github.com/apple/darwin-xnu/blob/main/osfmk/mach/task_info.h)
  and [the pinned libc bindings](https://github.com/rust-lang/libc/blob/0.2.190/src/unix/bsd/apple/mod.rs).

## Executed verification

On Windows, serial MSVC native testing with one Cargo job and task-local
TS export output passed both tests: bounded/malformed/overflow/page-size
parser cases and the actual current-process query. That query returned
6,889,472 bytes. This is an observed sample, not a memory-performance limit.

The three focused transport UI files passed 24 tests, including 11 new memory
cases: browser unavailability, MiB/GiB/zero, once-per-completion polling,
pending-query nonoverlap, failure/recovery, unsafe numeric values, unmount and
Strict Mode late-response isolation. The existing transport and position
tests remained passing on the actual shared WASM document backend. Desktop
TypeScript, scoped ESLint, strict desktop all-target Clippy, workspace Rust
formatting, changed-file Prettier and diff checks passed. `check-sim` reports the existing
1,982,055-byte WASM current; no generated artifact changed.

An actual T3 browser preview at 1280 × 800 showed the unavailable readout in
the toolbar without page horizontal overflow. Its measured rectangle was
x=1127.672..1163, y=48.172..62.828; the accessible label and scope tooltip were
present. Evidence screenshot:
`C:/Users/ewhee/.t3/userdata/browser-artifacts/browser-screenshot-localhost-muzjjhd9-c39b3882.png`.
Numeric polling behavior is exercised by the component tests with a substituted
backend response; browser preview does not prove native desktop IPC execution.

macOS and Linux native queries await their platform CI execution. Physical
device performance and aggregate child-process accounting are not claimed.

## Independent source review

The Standards review of source `0e81f508` against `125d709c` reported zero
findings. The separate Spec review found one P2: the accessible label said
only Host RAM and the value, although this contract promised the accounting
scope there too. The follow-up adds resident desktop-host meaning and the
webview/helper exclusions to every accessible label. An exact accessible-name
assertion and all 11 memory lifecycle tests pass after that correction;
TypeScript and scoped ESLint pass. Native accounting, IPC, units and polling
are unchanged. Both initial reviews were read-only source audits, not executed
platform tests.
