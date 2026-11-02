package adw

import (
	gtkbindings "codeberg.org/dergs/tonearm/pkg/schwifty/bindings/gtk"
	"codeberg.org/puregotk/puregotk/v4/adw"
)

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen StatusPage *adw.StatusPage adw

func (f StatusPage) Description(description string) StatusPage {
	return func() *adw.StatusPage {
		statusPage := f()
		statusPage.SetDescription(description)
		return statusPage
	}
}

func (f StatusPage) IconName(icon string) StatusPage {
	return func() *adw.StatusPage {
		statusPage := f()
		statusPage.SetIconName(icon)
		return statusPage
	}
}

func (f StatusPage) Loading() StatusPage {
	return func() *adw.StatusPage {
		statusPage := f()
		paintable := adw.NewSpinnerPaintable(&statusPage.Widget)
		defer paintable.Unref()
		statusPage.SetPaintable(paintable)
		return statusPage
	}
}

func (f StatusPage) Title(title string) StatusPage {
	return func() *adw.StatusPage {
		statusPage := f()
		statusPage.SetTitle(title)
		return statusPage
	}
}

func (f StatusPage) Child(widget any) StatusPage {
	return func() *adw.StatusPage {
		statusPage := f()
		statusPage.SetChild(gtkbindings.ResolveWidget(widget))
		return statusPage
	}
}
