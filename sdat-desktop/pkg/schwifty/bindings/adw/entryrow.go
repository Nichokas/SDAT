package adw

import "codeberg.org/puregotk/puregotk/v4/adw"

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen EntryRow *adw.EntryRow adw

func (f EntryRow) Title(title string) EntryRow {
	return func() *adw.EntryRow {
		row := f()
		row.SetTitle(title)
		return row
	}
}
