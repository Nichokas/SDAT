package adw

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/bindings/gtk"
	"codeberg.org/puregotk/puregotk/v4/adw"
)

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen ExpanderRow *adw.ExpanderRow adw

func (f ExpanderRow) Title(title string) ExpanderRow {
	return func() *adw.ExpanderRow {
		row := f()
		row.SetTitle(title)
		return row
	}
}

func (f ExpanderRow) Subtitle(subtitle string) ExpanderRow {
	return func() *adw.ExpanderRow {
		row := f()
		row.SetSubtitle(subtitle)
		return row
	}
}

func (f ExpanderRow) IconName(iconName string) ExpanderRow {
	return func() *adw.ExpanderRow {
		row := f()
		row.SetIconName(iconName)
		return row
	}
}

func (f ExpanderRow) Expanded(expanded bool) ExpanderRow {
	return func() *adw.ExpanderRow {
		row := f()
		row.SetExpanded(expanded)
		return row
	}
}

func (f ExpanderRow) EnableExpansion(enabled bool) ExpanderRow {
	return func() *adw.ExpanderRow {
		row := f()
		row.SetEnableExpansion(enabled)
		return row
	}
}

func (f ExpanderRow) ShowEnableSwitch(show bool) ExpanderRow {
	return func() *adw.ExpanderRow {
		row := f()
		row.SetShowEnableSwitch(show)
		return row
	}
}

func (f ExpanderRow) TitleLines(lines int32) ExpanderRow {
	return func() *adw.ExpanderRow {
		row := f()
		row.SetTitleLines(lines)
		return row
	}
}

func (f ExpanderRow) SubtitleLines(lines int32) ExpanderRow {
	return func() *adw.ExpanderRow {
		row := f()
		row.SetSubtitleLines(lines)
		return row
	}
}

func (f ExpanderRow) AddRow(child any) ExpanderRow {
	return func() *adw.ExpanderRow {
		row := f()
		row.AddRow(gtk.ResolveWidget(child))
		return row
	}
}

func (f ExpanderRow) AddPrefix(child any) ExpanderRow {
	return func() *adw.ExpanderRow {
		row := f()
		row.AddPrefix(gtk.ResolveWidget(child))
		return row
	}
}

func (f ExpanderRow) AddSuffix(child any) ExpanderRow {
	return func() *adw.ExpanderRow {
		row := f()
		row.AddSuffix(gtk.ResolveWidget(child))
		return row
	}
}
