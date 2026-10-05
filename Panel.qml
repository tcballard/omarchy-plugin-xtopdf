import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import qs.Commons
import qs.Ui as Ui

Item {
  id: root
  property var bar: null
  property Item anchorItem: null
  property bool opened: false
  property bool ready: false
  property bool busy: false
  property bool captured: false
  property bool failed: false
  property string status: "Open an X Article to begin."
  property string articleTitle: "Your next good read."
  property string byline: ""
  property string preview: ""
  property string savedPath: ""
  property string paper: "Letter"
  readonly property string helper: decodeURIComponent(Qt.resolvedUrl("bin/x-to-pdf").toString().replace(/^file:\/\//, ""))

  function open(payloadJson) {
    var p = ({})
    try { if (String(payloadJson || "{}").length <= 4096) p = JSON.parse(payloadJson || "{}") || ({}) } catch (e) {}
    if (!worker.running && typeof p.url === "string") urlField.text = p.url.slice(0, 2048)
    opened = true
  }
  // Hiding lets the user read/sign in in Chromium; End session stops the helper.
  function close() { opened = false }
  function stop() {
    if (worker.running) { worker.write('{"action":"cancel"}\n'); worker.running = false; reapTimer.restart() }
    ready = false; busy = false; captured = false
    status = "Session ended. Your PDFs and separate X sign-in are kept."
  }
  function start(demo) {
    if (worker.running) return
    ready = false; busy = true; captured = false; failed = false
    savedPath = ""; preview = ""; articleTitle = "Your next good read."; byline = ""
    status = "Opening the separate article browser…"
    worker.command = demo ? [helper, "--demo"] : [helper, "--session", urlField.text.trim()]
    worker.running = true
    launchTimer.restart()
  }
  function send(request) {
    if (!worker.running || busy) return
    busy = true; failed = false; savedPath = ""
    if (request.action === "capture") { captured = false; preview = ""; articleTitle = "Reading your article…"; byline = ""; status = "Reading the loaded article…" }
    else status = "Typesetting your PDF…"
    worker.write(JSON.stringify(request) + "\n")
  }
  function consume(line) {
    if (line.length > 120000) { stop(); failed = true; status = "Helper response exceeded the limit."; return }
    var e
    try { e = JSON.parse(line) } catch (error) { stop(); failed = true; status = "The helper returned an invalid response."; return }
    launchTimer.stop()
    busy = false
    status = String(e.message || "").slice(0, 500)
    if (e.event === "ready") ready = true
    else if (e.event === "captured") {
      captured = true; articleTitle = String(e.title || "X Article").slice(0, 4096)
      byline = String(e.byline || "").slice(0, 4096); preview = String(e.preview || "").slice(0, 16000)
    } else if (e.event === "saved") { savedPath = String(e.path || ""); captured = true }
    else if (e.event === "error") failed = true
  }
  Component.onDestruction: { if (worker.running) worker.running = false }

  Process {
    id: worker
    stdinEnabled: true
    clearEnvironment: true
    environment: ({HOME: null, XDG_DATA_HOME: null, XDG_RUNTIME_DIR: null, WAYLAND_DISPLAY: null,
      DISPLAY: null, XAUTHORITY: null, DBUS_SESSION_BUS_ADDRESS: null, LANG: null, LC_ALL: null,
      XDG_CURRENT_DESKTOP: null, PATH: "/usr/bin:/bin"})
    stdout: SplitParser { onRead: data => root.consume(data) }
    onExited: function(exitCode, exitStatus) {
      launchTimer.stop(); reapTimer.stop(); root.ready = false; root.busy = false
      if (exitCode !== 0 && !root.failed) { root.failed = true; root.status = "The helper could not run. Check that bin/x-to-pdf and Chromium are installed." }
    }
  }
  Timer {
    id: launchTimer; interval: 35000
    onTriggered: { root.stop(); root.failed = true; root.status = "The helper did not start. Check Chromium and the plugin binary." }
  }
  Timer { id: reapTimer; interval: 8000; onTriggered: if (worker.running) worker.signal(9) }
  Process { id: opener; command: ["/usr/bin/xdg-open", root.savedPath] }

  Ui.KeyboardPanel {
    id: popup
    anchorItem: root.anchorItem
    bar: root.bar
    owner: root
    open: root.opened
    focusTarget: panelFocus
    contentWidth: fittedContentWidth(Style.space(540))
    contentHeight: cappedContentHeight(Style.space(650))

    Flickable {
      id: panelFocus
      focus: true
      anchors.fill: parent
      contentWidth: width
      contentHeight: form.implicitHeight
      clip: true
      Keys.onEscapePressed: root.close()

      ColumnLayout {
        id: form
        width: parent.width
        spacing: Style.spacing.panelGap
        RowLayout {
          Layout.fillWidth: true
          ColumnLayout {
            Layout.fillWidth: true
            spacing: Style.space(3)
            Text { text: "X → PDF"; textFormat: Text.PlainText; color: Color.accent; font.pixelSize: Style.font.display; font.family: Style.font.family }
            Text { text: "Save articles. Read anywhere."; textFormat: Text.PlainText; color: Color.popups.text; font.pixelSize: Style.font.subtitle }
          }
          Ui.Button { text: "Close"; focusable: true; onClicked: root.close() }
        }
        Rectangle { Layout.fillWidth: true; height: 1; color: Color.popups.border }
        Text { text: "01  OPEN AN ARTICLE"; textFormat: Text.PlainText; color: Color.accent; font.pixelSize: Style.font.caption }
        Ui.TextField {
          id: urlField; Layout.fillWidth: true; placeholderText: "https://x.com/author/status/…"
          maximumLength: 2048; enabled: !worker.running
          Keys.onEscapePressed: root.close()
          onAccepted: root.start(false)
        }
        RowLayout {
          Ui.Button { text: "Open article"; bordered: true; focusable: true; enabled: !worker.running && urlField.text.trim() !== ""; opacity: enabled ? 1 : 0.45; onClicked: root.start(false) }
          Ui.Button { text: "Try a sample"; focusable: true; enabled: !worker.running; opacity: enabled ? 1 : 0.45; onClicked: root.start(true) }
          Item { Layout.fillWidth: true }
          Ui.Button { text: "End session"; focusable: true; visible: worker.running; onClicked: root.stop() }
        }
        Text {
          Layout.fillWidth: true; wrapMode: Text.WordWrap; textFormat: Text.PlainText
          text: root.status; color: root.failed ? Color.urgent : Color.popups.text; font.pixelSize: Style.font.body
        }
        Text { text: "02  CAPTURE & REVIEW"; textFormat: Text.PlainText; color: Color.accent; font.pixelSize: Style.font.caption }
        RowLayout {
          Layout.fillWidth: true
          Text { Layout.fillWidth: true; text: root.articleTitle; textFormat: Text.PlainText; color: Color.popups.text; font.pixelSize: Style.font.heading; maximumLineCount: 2; wrapMode: Text.Wrap; elide: Text.ElideRight }
          Ui.Button { text: root.busy ? "Working…" : "Capture"; bordered: true; focusable: true; enabled: root.ready && !root.busy; opacity: enabled ? 1 : 0.45; onClicked: root.send({action:"capture"}) }
        }
        Text { Layout.fillWidth: true; visible: root.byline !== ""; text: root.byline; textFormat: Text.PlainText; color: Color.popups.text; font.pixelSize: Style.font.caption; elide: Text.ElideRight }
        Rectangle {
          Layout.fillWidth: true; implicitHeight: Style.space(155)
          color: Qt.alpha(Color.foreground, 0.035); border.color: Color.popups.border; radius: Style.cornerRadius
          Flickable {
            anchors.fill: parent; anchors.margins: Style.space(12); clip: true; contentHeight: previewText.implicitHeight
            TextEdit {
              id: previewText; width: parent.width; readOnly: true; selectByMouse: true
              textFormat: TextEdit.PlainText; wrapMode: TextEdit.Wrap
              text: root.preview || "Once the full article is loaded in Chromium, come back here and click Capture. Check that the ending is present before saving."
              color: Color.popups.text; font.pixelSize: Style.font.body
              Keys.onEscapePressed: root.close()
            }
          }
        }
        Text { visible: root.preview.length >= 16000; text: "Text review is limited to 16,000 characters; export includes the full capture."; textFormat: Text.PlainText; Layout.fillWidth: true; wrapMode: Text.WordWrap; color: Color.popups.text; font.pixelSize: Style.font.caption }
        Text { text: "03  SAVE YOUR PDF"; textFormat: Text.PlainText; color: Color.accent; font.pixelSize: Style.font.caption }
        RowLayout {
          Text { text: "Two columns · 9 pt serif"; textFormat: Text.PlainText; color: Color.popups.text; font.pixelSize: Style.font.body; Layout.fillWidth: true }
          Ui.Button { text: "Letter"; selected: root.paper === "Letter"; bordered: true; focusable: true; onClicked: root.paper = "Letter" }
          Ui.Button { text: "A4"; selected: root.paper === "A4"; bordered: true; focusable: true; onClicked: root.paper = "A4" }
        }
        Ui.TextField {
          id: destination; Layout.fillWidth: true; maximumLength: 4096
          text: Quickshell.env("HOME") + "/Downloads/X-Article-" + Date.now() + ".pdf"
          placeholderText: "Absolute destination path ending in .pdf"
          Keys.onEscapePressed: root.close()
        }
        RowLayout {
          Ui.Button { text: "Save PDF"; bordered: true; selected: true; focusable: true; enabled: root.captured && root.ready && !root.busy; opacity: enabled ? 1 : 0.45; onClicked: root.send({action:"export", path:destination.text, paper:root.paper}) }
          Ui.Button { text: "Open saved PDF"; focusable: true; visible: root.savedPath !== ""; onClicked: { if (!opener.running) opener.running = true } }
        }
        Text {
          Layout.fillWidth: true; wrapMode: Text.WordWrap; textFormat: Text.PlainText
          text: "Local export · Separate X sign-in · Existing files are never replaced\nBased on Sahand Sojoodi’s X to PDF."
          color: Color.popups.text; opacity: 0.65; font.pixelSize: Style.font.caption
        }
      }
    }
  }
}
