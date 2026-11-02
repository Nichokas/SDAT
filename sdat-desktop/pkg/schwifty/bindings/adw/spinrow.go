package adw

import "codeberg.org/puregotk/puregotk/v4/adw"

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen SpinRow *adw.SpinRow adw

func (f SpinRow) Title(title string) SpinRow {
	return func() *adw.SpinRow {
		row := f()
		row.SetTitle(title)
		return row
	}
}

func (f SpinRow) Subtitle(subtitle string) SpinRow {
	return func() *adw.SpinRow {
		row := f()
		row.SetSubtitle(subtitle)
		return row
	}
}
