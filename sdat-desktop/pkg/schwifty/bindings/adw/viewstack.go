package adw

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/bindings/gtk"
	"codeberg.org/puregotk/puregotk/v4/adw"
)

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen ViewStack *adw.ViewStack adw

func (f ViewStack) AddTitledWithIcon(child any, name string, title string, icon string) ViewStack {
	return func() *adw.ViewStack {
		viewStack := f()
		viewStack.AddTitledWithIcon(gtk.ResolveWidget(child), name, title, icon)
		return viewStack
	}
}
