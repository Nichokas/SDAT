package adw

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/bindings/gtk"
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/puregotk/puregotk/v4/adw"
)

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen ActionRow *adw.ActionRow adw

func (f ActionRow) Title(title string) ActionRow {
	return func() *adw.ActionRow {
		row := f()
		row.SetTitle(title)
		return row
	}
}

func (f ActionRow) Subtitle(subtitle string) ActionRow {
	return func() *adw.ActionRow {
		row := f()
		row.SetSubtitle(subtitle)
		return row
	}
}

func (f ActionRow) IconName(iconName string) ActionRow {
	return func() *adw.ActionRow {
		row := f()
		row.SetIconName(iconName)
		return row
	}
}

func (f ActionRow) ActivatableChild(child any) ActionRow {
	return func() *adw.ActionRow {
		row := f()
		w := gtk.ResolveWidget(child)
		row.SetActivatableWidget(w)
		return row
	}
}

func (f ActionRow) ActionSuffix(child any) ActionRow {
	return func() *adw.ActionRow {
		row := f()
		w := gtk.ResolveWidget(child)
		row.AddSuffix(w)
		row.SetActivatableWidget(w)
		return row
	}
}

func (f ActionRow) ConnectActivated(cb func(adw.ActionRow)) ActionRow {
	return func() *adw.ActionRow {
		row := f()
		callback.HandleCallback(row.Object, "activated", cb)
		return row
	}
}
