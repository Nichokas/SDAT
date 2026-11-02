package adw

import "codeberg.org/puregotk/puregotk/v4/adw"

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen PasswordEntryRow *adw.PasswordEntryRow adw

func (f PasswordEntryRow) Title(title string) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		row := f()
		row.SetTitle(title)
		return row
	}
}
