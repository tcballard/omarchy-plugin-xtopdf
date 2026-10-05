<h1 align="center">X to PDF for Omarchy</h1>

<p align="center">X Articles, ready to read offline.</p>

<p align="center">
  <a href="https://github.com/tcballard/omarchy-badges"><img alt="Built for Omarchy: Plugin" height="24" src="https://raw.githubusercontent.com/tcballard/omarchy-badges/75975e5b5bf75e7ede3764bcd2950046f7abfe2c/badges/v1/omarchy-plugin.svg"></a>
</p>

> [!NOTE]
> **Built on [X to PDF](https://sojoodi.com/apps/XtoPDF/) by [Sahand Sojoodi](https://sojoodi.com/).**
> Sahand's Chrome extension is the original inspiration for this project.
> This Omarchy port is made with his permission and adapts his article extraction
> and print layout. [Original MIT licence and source attribution →](NOTICE.md)

Turn long-form X Articles into clean, two-column PDFs from your Omarchy bar.
Open an article, review the captured text, and save a local copy to read later.

- Keep headings, emphasis, links, lists, quotations and code in reading order.
- Choose Letter or A4 and save directly to your computer.
- Use a separate X sign-in without changing your everyday browser profile.

**v0.0.1 development preview.** Seven Rust tests and portable checks pass;
live Omarchy, signed-in X capture and PDF output still need XPS validation.
[Testing details](docs/QA.md).

**Try it:** [build and install the preview](#install-the-preview) on your Omarchy desktop:

```bash
./scripts/build
./scripts/install-local
```

*On-device preview: XPS screenshot pending.*

## Install the preview

From a source checkout:

```bash
sudo pacman -S --needed git rust base-devel chromium
git clone https://github.com/tcballard/omarchy-plugin-xtopdf.git
cd omarchy-plugin-xtopdf
./scripts/build
./scripts/install-local
```

The build creates the native helper and its checksum. The installer checks that
checksum, validates the plugin, copies runtime files
into your user plugin directory and enables it. It refuses to overwrite an
existing installation. It does not change Hyprland configuration or keybindings.
If you already have the shared preview ZIP, extract it and run
`./scripts/install-local` inside its `omarchy-x-to-pdf` directory after installing
Chromium. That bundle includes a prebuilt x86_64 helper. GitHub's source ZIP does
not include the helper; build it first using the steps above.
There is no marketplace listing for this development preview.

## Use it

1. Click **X → PDF** in the bar and paste the full article URL.
2. Click **Open article**. A separate Chromium window opens. Sign in to X there
   if needed, open the full Article and let it load. Your normal browser profile
   is never opened or copied.
3. Return to the panel and click **Capture**. Review the text, particularly the
   ending; X may load article content progressively. Capture again if needed.
4. Choose **Letter** or **A4**, edit the absolute output filename if desired,
   and click **Save PDF**. Then **Open saved PDF** to inspect the layout.

**Try a sample** exercises the same rendering pipeline using fictional data.
It still requires Chromium. Capture and export are explicit user actions.
Close, Escape or clicking outside hides the panel so you can interact with the
article browser. **End session** closes the helper and its browser. Disabling or
reloading the plugin also cancels its helper. Sessions expire after 15 minutes.
One capture session at a time is enforced across monitors.

## What it preserves

Headings, emphasis, HTTP(S) links, lists, quotes, code and repeated paragraphs.
Code stays in the original reading order. Output is two columns with 9 pt serif
body text. Times New Roman is used if installed; otherwise the system's Times
or serif fallback is used. A4 is an addition to upstream's Letter format.

Ordinary posts, threads, images, embedded video, interactive content and exact
nested-list numbering are outside this preview. The review pane shows up to
16,000 characters; the PDF contains the complete captured document. There is no
guarantee that X has loaded every block: compare with the original article.
X DOM changes or a sign-in restriction may prevent capture; the plugin does not
bypass either. Text review is not a visual PDF preview.

## Privacy and removal

Article loading uses a normal networked Chromium window. X and its page assets
still receive the requests needed to display that page. The plugin has no
conversion server, telemetry, cookie import or TCP debugging listener.
A private inherited pipe connects Rust to Chromium.
PDF rendering uses a fresh headless profile, blocks document network requests,
disables scripts and only receives generated, escaped article markup.

The dedicated X sign-in lives in
`${XDG_DATA_HOME:-~/.local/share}/x-to-pdf/chromium/`, protected by an owner-only
parent directory. Chromium owns that data; the helper does not read cookies.
Captured article text stays in memory. Saved PDFs are created with private
permissions, and existing files are never overwritten. Chromium may retain its
own cache/history in the dedicated profile until you clear it.

```bash
omarchy plugin remove io.github.tcballard.x-to-pdf
```

Saved PDFs and the separate Chromium profile remain after removal. To clear the
sign-in, end the session and delete only the `x-to-pdf` data folder above using
your file manager. Removing that folder does not touch your usual browser.

## Compatibility and development

Targets Omarchy's current Quattro hosted QML contracts as inspected on
5 October 2026. Requires Quickshell with `qs.Ui.KeyboardPanel`, `qs.Ui.Button`
and `qs.Ui.TextField`, Chromium and `xdg-open`. No live Omarchy version is yet
certified. The supplied binary was built on x86_64 Linux, glibc 2.39, Rust 1.88.0.

```bash
./tests/run
./scripts/build
node tests/build-browser-tests.cjs /tmp/xtopdf-dom-tests.html
chromium /tmp/xtopdf-dom-tests.html
```

The browser fixture page must show **ALL PASSED**. Its results were not run in
the build environment. Development checks need Rust, Node and Python 3 (only for
the manifest validator); there is no Python or Node runtime dependency.
The [CI workflow](https://github.com/tcballard/omarchy-plugin-xtopdf/actions)
runs portable checks and builds the helper. Live desktop checks remain separate;
see [the validation record](docs/QA.md).

Optional desktop binding, without modifying any config:

```bash
omarchy-shell shell summon io.github.tcballard.x-to-pdf '{}'
```
