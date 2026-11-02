package adw

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/bindings"
	"codeberg.org/puregotk/puregotk/v4/adw"
)

type ShortcutsSection func() *adw.ShortcutsSection

func (f ShortcutsSection) Add(child any) ShortcutsSection {
	return func() *adw.ShortcutsSection {
		page := f()
		page.Add(bindings.ResolveTo[*adw.ShortcutsItem, ShortcutsItem](child))
		return page
	}
}

func (f ShortcutsSection) Title(title string) ShortcutsSection {
	return func() *adw.ShortcutsSection {
		page := f()
		page.SetTitle(title)
		return page
	}
}
