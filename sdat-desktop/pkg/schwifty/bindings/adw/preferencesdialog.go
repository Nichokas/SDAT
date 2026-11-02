package adw

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/bindings"
	"codeberg.org/dergs/tonearm/pkg/schwifty/bindings/gtk"
	"codeberg.org/puregotk/puregotk/v4/adw"
)

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen PreferencesDialog *adw.PreferencesDialog adw

func (f PreferencesDialog) Add(child any) PreferencesDialog {
	return func() *adw.PreferencesDialog {
		dialog := f()
		dialog.Add(bindings.ResolveTo[*adw.PreferencesPage, PreferencesPage](child))
		return dialog
	}
}

func (f PreferencesDialog) Present(parent any) {
	resolvedParent := gtk.ResolveWidget(parent)
	if resolvedParent != nil {
		dialog := f()
		dialog.Present(resolvedParent)
	}
}
