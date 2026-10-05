import QtQuick
import qs.Ui

BarWidget {
  id: root
  moduleName: "io.github.tcballard.x-to-pdf"
  readonly property bool opened: panelLoader.item ? panelLoader.item.opened : false
  implicitWidth: button.implicitWidth
  implicitHeight: button.implicitHeight
  function injectPanel() {
    if (panelLoader.item) { panelLoader.item.bar = root.bar; panelLoader.item.anchorItem = button }
  }
  function open(payloadJson) { if (panelLoader.item) panelLoader.item.open(payloadJson) }
  function close() { if (panelLoader.item) panelLoader.item.close() }
  onBarChanged: injectPanel()
  Loader {
    id: panelLoader; active: true; visible: false; source: Qt.resolvedUrl("Panel.qml")
    onLoaded: { root.injectPanel(); Qt.callLater(root.injectPanel) }
  }
  WidgetButton {
    id: button; anchors.fill: parent; bar: root.bar
    text: root.vertical ? "PDF" : "X → PDF"
    tooltipText: "Save an X Article as a PDF"
    onPressed: function(mouseButton) {
      if (mouseButton === Qt.LeftButton) { if (root.opened) root.close(); else root.open("{}") }
    }
  }
}
