package adw

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/bindings"
	"codeberg.org/puregotk/puregotk/v4/adw"
)

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen PreferencesPage *adw.PreferencesPage adw

func (f PreferencesPage) Add(child any) PreferencesPage {
	return func() *adw.PreferencesPage {
		page := f()
		page.Add(bindings.ResolveTo[*adw.PreferencesGroup, PreferencesGroup](child))
		return page
	}
}

func (f PreferencesPage) IconName(iconName string) PreferencesPage {
	return func() *adw.PreferencesPage {
		page := f()
		page.SetIconName(iconName)
		return page
	}
}

func (f PreferencesPage) Title(title string) PreferencesPage {
	return func() *adw.PreferencesPage {
		page := f()
		page.SetTitle(title)
		return page
	}
}
