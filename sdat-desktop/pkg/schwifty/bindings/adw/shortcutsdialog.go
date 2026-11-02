package adw

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/bindings"
	"codeberg.org/dergs/tonearm/pkg/schwifty/bindings/gtk"
	"codeberg.org/puregotk/puregotk/v4/adw"
)

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen ShortcutsDialog *adw.ShortcutsDialog adw

func (f ShortcutsDialog) Add(child any) ShortcutsDialog {
	return func() *adw.ShortcutsDialog {
		dialog := f()
		dialog.Add(bindings.ResolveTo[*adw.ShortcutsSection, ShortcutsSection](child))
		return dialog
	}
}

func (f ShortcutsDialog) Present(parent any) {
	resolvedParent := gtk.ResolveWidget(parent)
	if resolvedParent != nil {
		dialog := f()
		dialog.Present(resolvedParent)
	}
}
