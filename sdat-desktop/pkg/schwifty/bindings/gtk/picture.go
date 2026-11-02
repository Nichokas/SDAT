package gtk

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/gdk"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen Picture *gtk.Picture gtk

func (f Picture) CanShrink(b bool) Picture {
	return func() *gtk.Picture {
		picture := f()
		picture.SetCanShrink(b)
		return picture
	}
}

func (f Picture) BindPaintable(state *state.State[Paintable]) Picture {
	return func() *gtk.Picture {
		var callbackId string
		var ref weak.WidgetRef
		return f.ConnectRealize(func(w gtk.Widget) {
			ref = weak.NewWidgetRef(&w)
			callbackId = state.AddCallback(func(newValue Paintable) {
				newValue.Ref()
				callback.OnMainThreadOncePure(func() {
					defer newValue.Unref()
					if obj := ref.Get(); obj != nil {
						defer obj.Unref()
						gtk.PictureNewFromInternalPtr(obj.Ptr).SetPaintable(newValue)
					}
				})
			})
		}).ConnectUnrealize(func(w gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}

func (f Picture) ContentFit(fit gtk.ContentFit) Picture {
	return func() *gtk.Picture {
		picture := f()
		picture.SetContentFit(fit)
		return picture
	}
}

func (f Picture) FromPaintable(paintable gdk.Paintable) Picture {
	return func() *gtk.Picture {
		picture := f()
		picture.SetPaintable(paintable)
		return picture
	}
}
