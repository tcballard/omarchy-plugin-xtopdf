# Attribution and source provenance

X to PDF for Omarchy is Tom Ballard's native shell port of **X to PDF** by
**Sahand Sojoodi**: https://sojoodi.com/apps/XtoPDF/.
Tom confirmed permission to port the extension on 5 October 2026.
The downloaded extension also carries the MIT license, retained verbatim in
`licenses/XtoPDF-MIT.txt` (copyright 2026 X to PDF contributors).

Inspected upstream: **1.0.1**, downloaded 5 October 2026 from:
https://sojoodi.com/apps/XtoPDF/downloads/x-to-pdf-v1.0.1.zip

SHA-256 of upstream ZIP:
`04e2ad2e04c1ef9ced6773015fa3e025a363b9b81f652af38492c42611dec14f`

- `assets/print.css` is copied from upstream without modification.
- `assets/extract.js` adapts upstream's selectors and extraction approach.
  Changes preserve document order and repeated paragraphs, preserve ordered
  lists, skip active elements, and return structured text instead of HTML.
- QML UI, Rust helper, IPC protocol and tests are new port code.
- No upstream icon or marketing image is used.
- Rust dependency license notices accompany the preview under
  `licenses/dependencies/`; `Cargo.lock` records the resolved versions.

Omarchy itself and Chromium are separate dependencies. This is an independent
port, not an official Omarchy or X product.
