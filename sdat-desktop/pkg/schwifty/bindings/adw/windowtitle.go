package adw

import "codeberg.org/puregotk/puregotk/v4/adw"

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen WindowTitle *adw.WindowTitle adw

func (f WindowTitle) Title(title string) WindowTitle {
	return func() *adw.WindowTitle {
		wt := f()
		wt.SetTitle(title)
		return wt
	}
}

func (f WindowTitle) SubTitle(subtitle string) WindowTitle {
	return func() *adw.WindowTitle {
		wt := f()
		wt.SetSubtitle(subtitle)
		return wt
	}
}
