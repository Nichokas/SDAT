package adw

import "codeberg.org/puregotk/puregotk/v4/adw"

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen ButtonRow *adw.ButtonRow adw

func (f ButtonRow) Title(title string) ButtonRow {
	return func() *adw.ButtonRow {
		row := f()
		row.SetTitle(title)
		return row
	}
}

func (f ButtonRow) StartIconName(name string) ButtonRow {
	return func() *adw.ButtonRow {
		row := f()
		row.SetStartIconName(name)
		return row
	}
}

func (f ButtonRow) EndIconName(name string) ButtonRow {
	return func() *adw.ButtonRow {
		row := f()
		row.SetEndIconName(name)
		return row
	}
}

func (f ButtonRow) ActionName(name string) ButtonRow {
	return func() *adw.ButtonRow {
		row := f()
		row.SetActionName(name)
		return row
	}
}

func (f ButtonRow) DetailedActionName(name string) ButtonRow {
	return func() *adw.ButtonRow {
		row := f()
		row.SetDetailedActionName(name)
		return row
	}
}
