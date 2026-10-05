# Port design — 5 October 2026

Working distribution assumption: independently maintained, reviewable preview;
public hosting and marketplace submission are later steps, not prerequisites.
Permanent ID: `io.github.tcballard.x-to-pdf`.

## Scope and architecture

The first vertical slice is an accessible long-form X Article → explicit capture
→ native text review → two-column local PDF. `bar-widget` is the only declared
kind. Its private panel forwards open/close/opened through the widget so host
summon/hide routing works. The built-in KeyboardPanel handles positioning on the
invoking bar/monitor, dismissal, theme tokens and keyboard focus. There is no
second Quickshell process and no full desktop app workspace.

Per-widget state: open/hidden state, user-entered URL/output path, paper choice,
status and capture preview. A Rust process owns captured structured content and
the article browser. A file lock prevents overlapping sessions on multiple
monitors. The helper lives while the panel is hidden so users can sign in/read;
End session, plugin destruction, stdin EOF, SIGTERM or the 15-minute deadline
ends it. Chromium is a distinct process group. Cleanup signals that group,
retains the leader until escalation finishes, then reaps it.

Persistent state: only a dedicated Chromium profile in XDG_DATA_HOME/x-to-pdf.
Session lock uses flock and O_NOFOLLOW. Chromium owns sign-in state. No credentials
are read, copied or passed through QML. No browser control socket listens on TCP.
The helper inherits an explicit desktop environment from QML and launches only
known Chromium executable locations. No shell interpolation.

Command stdin is bounded JSON: capture; export(path, paper); cancel. Output is
bounded JSON events, with only a capped plain-text preview passed to QML.
Capture uses an isolated JS world and the upstream article selectors. Rust
validates the source URL and renders an allowlisted structured document; arbitrary
article HTML never reaches the renderer. Renderer uses a fresh headless Chromium
profile, blocked document URLs, disabled script execution and restrictive CSP.

Output is written to a private temporary file in the chosen destination directory,
fsynced and persisted with no-clobber semantics. Final filename must be absolute
and end in .pdf. Removing the plugin preserves user documents and sign-in state.

## Prior art and dependencies

Reviewed XtoPDF 1.0.1 ZIP (MIT): exact requested functionality, with a Chrome
extension UI/Chrome storage boundary. Adapt the extractor and layout; replace
extension interaction/storage with hosted QML and a Rust process. Original
attribution is retained.
Bounded public repository search surfaced PDF preview/signing/editing plugins
(omarchy-quick-look, pdfseal, pdfstudio, pdfx), but did not establish an equivalent
long-form X Article exporter. Marketplace's fetched page exposed no usable
client-loaded listing data, so no uniqueness claim is made.

Evaluated Omakit Run/Store block documentation. They bring a Python runtime and
copy supervised subprocess/store machinery; this port already needs a Rust
helper that owns both the private Chromium pipe and process cleanup. Use that
boundary, with transport and cancellation checks, rather than a second supervisor.
Runtime: Quickshell/Omarchy, Chromium, xdg-open. No privileged runtime actions.
Network: X and the resources loaded by its browser page. No conversion service.

## Acceptance and remaining checks

- Fictional sample renders selectable text with preserved order and links.
- A real, accessible X Article captures its actual ending and formatting.
- Repeated capture, failed capture, existing output, cancellation and missing
  dependencies produce explicit states; no silent fallback to ordinary posts.
- Open/reopen/Escape/outside dismissal, four bar edges, theme changes, hot reload,
  monitor placement and disablement must be exercised on Omarchy.
- No session or renderer processes remain after cancellation/disablement.
- Complete live validation before a release or marketplace submission.

Deferred: extension bridge, automatic use of existing signed-in tabs, arbitrary
web pages, threads, images, visual in-panel PDF preview, batch export, packaging
into a pacman repository and marketplace submission.
