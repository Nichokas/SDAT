package adw

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type SpinRow func() *adw.SpinRow

func (f SpinRow) AddController(controller *gtk.EventController) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f SpinRow) ConnectConstruct(cb func(*adw.SpinRow)) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f SpinRow) ConnectDestroy(cb func(gtk.Widget)) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f SpinRow) ConnectHide(cb func(gtk.Widget)) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f SpinRow) ConnectMap(cb func(gtk.Widget)) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f SpinRow) ConnectRealize(cb func(gtk.Widget)) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f SpinRow) ConnectShow(cb func(gtk.Widget)) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f SpinRow) ConnectUnmap(cb func(gtk.Widget)) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f SpinRow) ConnectUnrealize(cb func(gtk.Widget)) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f SpinRow) Controller(controller *gtk.EventController) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f SpinRow) Focusable(focusable bool) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f SpinRow) FocusOnClick(focusOnClick bool) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f SpinRow) HAlign(align gtk.Align) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f SpinRow) HExpand(expand bool) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f SpinRow) HMargin(horizontal int32) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f SpinRow) Margin(margin int32) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f SpinRow) MarginBottom(bottom int32) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f SpinRow) MarginEnd(end int32) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f SpinRow) MarginStart(start int32) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f SpinRow) MarginTop(top int32) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f SpinRow) Opacity(opacity float64) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f SpinRow) Overflow(overflow gtk.Overflow) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f SpinRow) Sensitive(sensitive bool) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f SpinRow) SizeRequest(width, height int32) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f SpinRow) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f SpinRow) VAlign(align gtk.Align) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f SpinRow) VExpand(expand bool) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f SpinRow) Visible(visible bool) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f SpinRow) VMargin(vertical int32) SpinRow {
	return func() *adw.SpinRow {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f SpinRow) Background(color string) SpinRow {
	return func() *adw.SpinRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f SpinRow) CornerRadius(radius int) SpinRow {
	return func() *adw.SpinRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f SpinRow) CSS(css string) SpinRow {
	return func() *adw.SpinRow {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f SpinRow) BindCSSClass(state *state.State[string]) SpinRow {
	return func() *adw.SpinRow {
		var callbackId string
		var ref weak.WidgetRef
		return f.ConnectRealize(func(w gtk.Widget) {
			ref = weak.NewWidgetRef(&w)
			callbackId = state.AddCallback(func(newValue string) {
				oldValue := state.Value()
				callback.OnMainThreadOncePure(func() {
					if obj := ref.Get(); obj != nil {
						defer obj.Unref()

						w := gtk.WidgetNewFromInternalPtr(obj.Ptr)
						styleContext := w.GetStyleContext()
						defer styleContext.Unref()

						styleContext.RemoveClass(oldValue)
						styleContext.AddClass(newValue)
					}
				})
			})
		}).ConnectUnrealize(func(w gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}

func (f SpinRow) WithCSSClass(className string) SpinRow {
	return func() *adw.SpinRow {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f SpinRow) CSSWithCallback(cb func(elementName string) string) SpinRow {
	return func() *adw.SpinRow {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *adw.SpinRow) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f SpinRow) HPadding(padding int) SpinRow {
	return func() *adw.SpinRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f SpinRow) MinHeight(minHeight int) SpinRow {
	return func() *adw.SpinRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f SpinRow) MinWidth(minWidth int) SpinRow {
	return func() *adw.SpinRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f SpinRow) Padding(padding int) SpinRow {
	return func() *adw.SpinRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f SpinRow) PaddingBottom(padding int) SpinRow {
	return func() *adw.SpinRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f SpinRow) PaddingEnd(padding int) SpinRow {
	return func() *adw.SpinRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f SpinRow) PaddingStart(padding int) SpinRow {
	return func() *adw.SpinRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f SpinRow) PaddingTop(padding int) SpinRow {
	return func() *adw.SpinRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f SpinRow) VPadding(padding int) SpinRow {
	return func() *adw.SpinRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f SpinRow) BindVisible(state *state.State[bool]) SpinRow {
	return func() *adw.SpinRow {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *adw.SpinRow) {
			ref = weak.NewObjectRef(&w.Widget)
			callbackId = state.AddCallback(func(newValue bool) {
				callback.OnMainThreadOncePure(func() {
					if obj := ref.Get(); obj != nil {
						defer obj.Unref()
						gtk.WidgetNewFromInternalPtr(obj.Ptr).SetVisible(newValue)
					}
				})
			})
		}).ConnectDestroy(func(w gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}

func (f SpinRow) BindHMargin(state *state.State[int32]) SpinRow {
	return func() *adw.SpinRow {
		var callbackId string
		var ref weak.WidgetRef
		return f.ConnectRealize(func(w gtk.Widget) {
			ref = weak.NewWidgetRef(&w)
			callbackId = state.AddCallback(func(newValue int32) {
				callback.OnMainThreadOncePure(func() {
					if obj := ref.Get(); obj != nil {
						defer obj.Unref()
						gtk.WidgetNewFromInternalPtr(obj.Ptr).SetMarginEnd(newValue)
						gtk.WidgetNewFromInternalPtr(obj.Ptr).SetMarginStart(newValue)
					}
				})
			})
		}).ConnectUnrealize(func(w gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}

func (f SpinRow) BindMargin(state *state.State[int32]) SpinRow {
	return func() *adw.SpinRow {
		var callbackId string
		var ref weak.WidgetRef
		return f.ConnectRealize(func(w gtk.Widget) {
			ref = weak.NewWidgetRef(&w)
			callbackId = state.AddCallback(func(newValue int32) {
				callback.OnMainThreadOncePure(func() {
					if obj := ref.Get(); obj != nil {
						defer obj.Unref()
						gtk.WidgetNewFromInternalPtr(obj.Ptr).SetMarginBottom(newValue)
						gtk.WidgetNewFromInternalPtr(obj.Ptr).SetMarginEnd(newValue)
						gtk.WidgetNewFromInternalPtr(obj.Ptr).SetMarginStart(newValue)
						gtk.WidgetNewFromInternalPtr(obj.Ptr).SetMarginTop(newValue)
					}
				})
			})
		}).ConnectUnrealize(func(gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}

func (f SpinRow) BindMarginBottom(state *state.State[int32]) SpinRow {
	return func() *adw.SpinRow {
		var callbackId string
		var ref weak.WidgetRef
		return f.ConnectRealize(func(w gtk.Widget) {
			ref = weak.NewWidgetRef(&w)
			callbackId = state.AddCallback(func(newValue int32) {
				callback.OnMainThreadOncePure(func() {
					if obj := ref.Get(); obj != nil {
						defer obj.Unref()
						gtk.WidgetNewFromInternalPtr(obj.Ptr).SetMarginBottom(newValue)
					}
				})
			})
		}).ConnectUnrealize(func(w gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}

func (f SpinRow) BindMarginEnd(state *state.State[int32]) SpinRow {
	return func() *adw.SpinRow {
		var callbackId string
		var ref weak.WidgetRef
		return f.ConnectRealize(func(w gtk.Widget) {
			ref = weak.NewWidgetRef(&w)
			callbackId = state.AddCallback(func(newValue int32) {
				callback.OnMainThreadOncePure(func() {
					if obj := ref.Get(); obj != nil {
						defer obj.Unref()
						gtk.WidgetNewFromInternalPtr(obj.Ptr).SetMarginEnd(newValue)
					}
				})
			})
		}).ConnectUnrealize(func(w gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}

func (f SpinRow) BindMarginStart(state *state.State[int32]) SpinRow {
	return func() *adw.SpinRow {
		var callbackId string
		var ref weak.WidgetRef
		return f.ConnectRealize(func(w gtk.Widget) {
			ref = weak.NewWidgetRef(&w)
			callbackId = state.AddCallback(func(newValue int32) {
				callback.OnMainThreadOncePure(func() {
					if obj := ref.Get(); obj != nil {
						defer obj.Unref()
						gtk.WidgetNewFromInternalPtr(obj.Ptr).SetMarginStart(newValue)
					}
				})
			})
		}).ConnectUnrealize(func(w gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}

func (f SpinRow) BindMarginTop(state *state.State[int32]) SpinRow {
	return func() *adw.SpinRow {
		var callbackId string
		var ref weak.WidgetRef
		return f.ConnectRealize(func(w gtk.Widget) {
			ref = weak.NewWidgetRef(&w)
			callbackId = state.AddCallback(func(newValue int32) {
				callback.OnMainThreadOncePure(func() {
					if obj := ref.Get(); obj != nil {
						defer obj.Unref()
						gtk.WidgetNewFromInternalPtr(obj.Ptr).SetMarginTop(newValue)
					}
				})
			})
		}).ConnectUnrealize(func(w gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}

func (f SpinRow) BindSensitive(state *state.State[bool]) SpinRow {
	return func() *adw.SpinRow {
		var callbackId string
		var ref weak.WidgetRef
		return f.ConnectRealize(func(w gtk.Widget) {
			ref = weak.NewWidgetRef(&w)
			callbackId = state.AddCallback(func(newValue bool) {
				callback.OnMainThreadOncePure(func() {
					if obj := ref.Get(); obj != nil {
						defer obj.Unref()
						gtk.WidgetNewFromInternalPtr(obj.Ptr).SetSensitive(newValue)
					}
				})
			})
		}).ConnectUnrealize(func(w gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}
