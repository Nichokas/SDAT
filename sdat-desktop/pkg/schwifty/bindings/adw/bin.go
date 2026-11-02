package adw

import (
	gtkbindings "codeberg.org/dergs/tonearm/pkg/schwifty/bindings/gtk"
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen Bin *adw.Bin adw

func (f Bin) BindChild(state *state.State[any]) Bin {
	return func() *adw.Bin {
		var callbackId string
		var ref weak.WidgetRef
		return f.ConnectRealize(func(w gtk.Widget) {
			ref = weak.NewWidgetRef(&w)
			callbackId = state.AddCallback(func(newValue any) {
				widget := gtkbindings.ResolveWidgetOnMain(newValue)
				widget.Ref()

				callback.OnMainThreadOncePure(func() {
					defer widget.Unref()
					if obj := ref.Get(); obj != nil {
						defer obj.Unref()
						bin := adw.BinNewFromInternalPtr(obj.Ptr)
						if widget == nil {
							bin.SetChild(nil)
						} else {
							bin.SetChild(widget)
						}
					}
				})
			})
		}).ConnectUnrealize(func(w gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}

func (f Bin) Child(widget any) Bin {
	return func() *adw.Bin {
		bin := f()
		bin.SetChild(gtkbindings.ResolveWidget(widget))
		return bin
	}
}
