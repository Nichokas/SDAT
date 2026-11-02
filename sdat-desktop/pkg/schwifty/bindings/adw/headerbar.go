package adw

import (
	gtkbindings "codeberg.org/dergs/tonearm/pkg/schwifty/bindings/gtk"
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen HeaderBar *adw.HeaderBar adw

func (f HeaderBar) BindDecorationLayout(state *state.State[string]) HeaderBar {
	return func() *adw.HeaderBar {
		var callbackId string
		var ref weak.WidgetRef
		return f.ConnectConstruct(func(w *adw.HeaderBar) {
			ref = weak.NewWidgetRef(&w.Widget)
			callbackId = state.AddCallback(func(newValue string) {
				callback.OnMainThreadOncePure(func() {
					if obj := ref.Get(); obj != nil {
						defer obj.Unref()
						adw.HeaderBarNewFromInternalPtr(obj.Ptr).SetDecorationLayout(newValue)
					}
				})
			})
		}).ConnectDestroy(func(w gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}

func (f HeaderBar) CenteringPolicy(policy adw.CenteringPolicy) HeaderBar {
	return func() *adw.HeaderBar {
		hb := f()
		hb.SetCenteringPolicy(policy)
		return hb
	}
}

func (f HeaderBar) DecorationLayout(layout string) HeaderBar {
	return func() *adw.HeaderBar {
		hb := f()
		hb.SetDecorationLayout(layout)
		return hb
	}
}

func (f HeaderBar) PackEnd(widget ...any) HeaderBar {
	return func() *adw.HeaderBar {
		hb := f()
		for _, w := range widget {
			hb.PackEnd(gtkbindings.ResolveWidget(w))
		}
		return hb
	}
}

func (f HeaderBar) PackStart(widget ...any) HeaderBar {
	return func() *adw.HeaderBar {
		hb := f()
		for _, w := range widget {
			hb.PackStart(gtkbindings.ResolveWidget(w))
		}
		return hb
	}
}

func (f HeaderBar) ShowBackButton(show bool) HeaderBar {
	return func() *adw.HeaderBar {
		hb := f()
		hb.SetShowBackButton(show)
		return hb
	}
}

func (f HeaderBar) ShowEndTitleButtons(show bool) HeaderBar {
	return func() *adw.HeaderBar {
		hb := f()
		hb.SetShowEndTitleButtons(show)
		return hb
	}
}

func (f HeaderBar) ShowStartTitleButtons(show bool) HeaderBar {
	return func() *adw.HeaderBar {
		hb := f()
		hb.SetShowStartTitleButtons(show)
		return hb
	}
}

func (f HeaderBar) TitleWidget(widget any) HeaderBar {
	return func() *adw.HeaderBar {
		hb := f()
		hb.SetTitleWidget(gtkbindings.ResolveWidget(widget))
		return hb
	}
}
