# Preview verification

Date: 5 October 2026. x86_64 Linux; Rust 1.88.0; Node 24.19.0; glibc 2.39.

## Passing local evidence

- `cargo test --locked --offline`: 7 tests. URL boundaries; structured HTML
  escaping/list/code/repeated-paragraph semantics; no-clobber PDF writes including
  symlink destinations; bounded command parsing; profile symlink rejection;
  private FD 3/4 protocol framing with unsolicited events and split responses;
  closed pipe errors and leader reaping.
- `cargo clippy --locked --offline --all-targets -- -D warnings`.
- `cargo fmt --check` and JS syntax checks.
- Rust release build and `--render-fixture` HTML output.
- Missing Chromium: `--check` exits 1 with a useful JSON error.
- Portable plugin manifest validation. Advisory scanner reports runtime process
  and bundled binary capabilities requiring review; it is not security certification.

The CDP test peer is a mock, not Chromium. It does not establish PDF fidelity,
real browser startup, X authentication or actual desktop process cleanup.

## Unrun checks — required before calling this a release

There is no Chromium/Qt/Omarchy desktop in the build container. The cloud browser
refused a local-file fixture URL under its URL security policy; no workaround
was used. Browser DOM fixtures, live QML loading, and actual PDF output remain
unverified. Omakit/marketplace baseline and remote CI have not run.

## XPS acceptance

1. Run `omarchy-version`, `chromium --version`, and record versions.
2. Install via `./scripts/install-local`; bar label appears with no shell errors.
3. Open/close/reopen. Escape and outside click dismiss. Test Tab and Enter;
   try each bar edge and a second monitor. Theme/font-size changes stay usable.
4. Try a sample → Capture → Save PDF. Inspect two columns, order, repeated
   paragraphs, selectable text, clickable HTTPS link and harmless markup text.
5. Repeat with A4. Attempt the same filename: previous bytes remain unchanged.
6. Open a real X Article. Sign in only in its separate Chromium window. Scroll
   through the full article, return to the panel, Capture, compare title/body/end,
   then export and compare the PDF. An ordinary post should fail clearly.
7. Close the article browser, try Capture, end session and retry. Hide/reopen
   during loading/export. End session while idle/capturing/printing; verify no
   dedicated Chromium processes survive. Repeat during plugin disable/reload.
8. Confirm a second monitor cannot start a second profile session.
9. Run DOM fixtures with `node tests/build-browser-tests.cjs /tmp/xtopdf-dom-tests.html`
   then `chromium /tmp/xtopdf-dom-tests.html`. Expect ALL PASSED.
10. Remove plugin. Check normal browser/Hyprland settings are unchanged and PDFs
    remain. Dedicated sign-in/profile remains only in the documented data folder.

If a live check fails, retain the exact source revision, versions, visible error
and reproduction. Do not publish v0.1.0 or claim marketplace readiness yet.
