package ui

import (
	"codeberg.org/dergs/tonearm/internal/gettext"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
)

func (w *Window) PresentShortcuts() {
	ShortcutsDialog(
		ShortcutsSection(
			ShortcutsItemFromAction(gettext.Get("Close"), "win.close"),
			ShortcutsItemFromAction(gettext.Get("Quit"), "app.quit"),
			ShortcutsItemFromAction(gettext.Get("Side Pane"), "win.toggle-sidebar"),
			ShortcutsItemFromAction(gettext.Get("Main Menu"), "win.main-menu"),
			ShortcutsItemFromAction(gettext.Get("Keyboard Shortcuts"), "app.shortcuts"),
			ShortcutsItemFromAction(gettext.Get("Preferences"), "app.preferences"),
		).Title(gettext.Get("Basic Shortcuts")),
		ShortcutsSection(
			ShortcutsItemFromAction(gettext.Get("Back"), "win.navigate-back"),
			ShortcutsItemFromAction(gettext.Get("Search"), "win.search"),
		).Title(gettext.Get("Navigation")),
	).Present(w)
}
