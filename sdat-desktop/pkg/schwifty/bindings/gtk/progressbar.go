package gtk

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"codeberg.org/puregotk/puregotk/v4/pango"
)

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen ProgressBar *gtk.ProgressBar gtk

func (f ProgressBar) Fraction(fraction float64) ProgressBar {
	return func() *gtk.ProgressBar {
		pb := f()
		pb.SetFraction(fraction)
		return pb
	}
}

func (f ProgressBar) BindFraction(state *state.State[float64]) ProgressBar {
	return func() *gtk.ProgressBar {
		var callbackId string
		var ref weak.WidgetRef
		return f.ConnectRealize(func(w gtk.Widget) {
			ref = weak.NewWidgetRef(&w)
			callbackId = state.AddCallback(func(newValue float64) {
				callback.OnMainThreadOncePure(func() {
					if obj := ref.Get(); obj != nil {
						defer obj.Unref()
						gtk.ProgressBarNewFromInternalPtr(obj.Ptr).SetFraction(newValue)
					}
				})
			})
		}).ConnectUnrealize(func(w gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}

func (f ProgressBar) Inverted(inverted bool) ProgressBar {
	return func() *gtk.ProgressBar {
		pb := f()
		pb.SetInverted(inverted)
		return pb
	}
}

func (f ProgressBar) PulseStep(fraction float64) ProgressBar {
	return func() *gtk.ProgressBar {
		pb := f()
		pb.SetPulseStep(fraction)
		return pb
	}
}

func (f ProgressBar) ShowText(showText bool) ProgressBar {
	return func() *gtk.ProgressBar {
		pb := f()
		pb.SetShowText(showText)
		return pb
	}
}

func (f ProgressBar) Text(text string) ProgressBar {
	return func() *gtk.ProgressBar {
		pb := f()
		pb.SetText(text)
		return pb
	}
}

func (f ProgressBar) BindText(state *state.State[string]) ProgressBar {
	return func() *gtk.ProgressBar {
		var callbackId string
		var ref weak.WidgetRef
		return f.ConnectRealize(func(w gtk.Widget) {
			ref = weak.NewWidgetRef(&w)
			callbackId = state.AddCallback(func(newValue string) {
				callback.OnMainThreadOncePure(func() {
					if obj := ref.Get(); obj != nil {
						defer obj.Unref()
						gtk.ProgressBarNewFromInternalPtr(obj.Ptr).SetText(newValue)
					}
				})
			})
		}).ConnectUnrealize(func(w gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}

func (f ProgressBar) Ellipsize(mode pango.EllipsizeMode) ProgressBar {
	return func() *gtk.ProgressBar {
		pb := f()
		pb.SetEllipsize(mode)
		return pb
	}
}
