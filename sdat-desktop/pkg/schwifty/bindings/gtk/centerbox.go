package gtk

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen CenterBox *gtk.CenterBox gtk

func (f CenterBox) BindCenterWidget(state *state.State[any]) CenterBox {
	return func() *gtk.CenterBox {
		var callbackId string
		var ref weak.WidgetRef
		return f.ConnectRealize(func(w gtk.Widget) {
			ref = weak.NewWidgetRef(&w)
			callbackId = state.AddCallback(func(newValue any) {
				widget := ResolveWidget(newValue)
				if widget == nil {
					callback.OnMainThreadOncePure(func() {
						if obj := ref.Get(); obj != nil {
							defer obj.Unref()
							gtk.CenterBoxNewFromInternalPtr(obj.Ptr).SetCenterWidget(nil)
						}
					})
				} else {
					widget.Ref()
					callback.OnMainThreadOncePure(func() {
						defer widget.Unref()
						if obj := ref.Get(); obj != nil {
							defer obj.Unref()
							gtk.CenterBoxNewFromInternalPtr(obj.Ptr).SetCenterWidget(widget)
						}
					})
				}
			})
		}).ConnectUnrealize(func(w gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}

func (f CenterBox) CenterWidget(child any) CenterBox {
	return func() *gtk.CenterBox {
		centerBox := f()
		centerBox.SetCenterWidget(ResolveWidget(child))
		return centerBox
	}
}

func (f CenterBox) EndWidget(child any) CenterBox {
	return func() *gtk.CenterBox {
		centerBox := f()
		centerBox.SetEndWidget(ResolveWidget(child))
		return centerBox
	}
}

func (f CenterBox) Orientation(orientation gtk.Orientation) CenterBox {
	return func() *gtk.CenterBox {
		centerBox := f()
		centerBox.SetOrientation(orientation)
		return centerBox
	}
}

func (f CenterBox) StartWidget(child any) CenterBox {
	return func() *gtk.CenterBox {
		centerBox := f()
		centerBox.SetStartWidget(ResolveWidget(child))
		return centerBox
	}
}
