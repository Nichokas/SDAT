package adw

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type ComboRow func() *adw.ComboRow

func (f ComboRow) AddController(controller *gtk.EventController) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f ComboRow) ConnectConstruct(cb func(*adw.ComboRow)) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f ComboRow) ConnectDestroy(cb func(gtk.Widget)) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f ComboRow) ConnectHide(cb func(gtk.Widget)) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f ComboRow) ConnectMap(cb func(gtk.Widget)) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f ComboRow) ConnectRealize(cb func(gtk.Widget)) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f ComboRow) ConnectShow(cb func(gtk.Widget)) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f ComboRow) ConnectUnmap(cb func(gtk.Widget)) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f ComboRow) ConnectUnrealize(cb func(gtk.Widget)) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f ComboRow) Controller(controller *gtk.EventController) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f ComboRow) Focusable(focusable bool) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f ComboRow) FocusOnClick(focusOnClick bool) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f ComboRow) HAlign(align gtk.Align) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f ComboRow) HExpand(expand bool) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f ComboRow) HMargin(horizontal int32) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f ComboRow) Margin(margin int32) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f ComboRow) MarginBottom(bottom int32) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f ComboRow) MarginEnd(end int32) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f ComboRow) MarginStart(start int32) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f ComboRow) MarginTop(top int32) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f ComboRow) Opacity(opacity float64) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f ComboRow) Overflow(overflow gtk.Overflow) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f ComboRow) Sensitive(sensitive bool) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f ComboRow) SizeRequest(width, height int32) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f ComboRow) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f ComboRow) VAlign(align gtk.Align) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f ComboRow) VExpand(expand bool) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f ComboRow) Visible(visible bool) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f ComboRow) VMargin(vertical int32) ComboRow {
	return func() *adw.ComboRow {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f ComboRow) Background(color string) ComboRow {
	return func() *adw.ComboRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f ComboRow) CornerRadius(radius int) ComboRow {
	return func() *adw.ComboRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f ComboRow) CSS(css string) ComboRow {
	return func() *adw.ComboRow {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f ComboRow) BindCSSClass(state *state.State[string]) ComboRow {
	return func() *adw.ComboRow {
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

func (f ComboRow) WithCSSClass(className string) ComboRow {
	return func() *adw.ComboRow {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f ComboRow) CSSWithCallback(cb func(elementName string) string) ComboRow {
	return func() *adw.ComboRow {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *adw.ComboRow) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f ComboRow) HPadding(padding int) ComboRow {
	return func() *adw.ComboRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f ComboRow) MinHeight(minHeight int) ComboRow {
	return func() *adw.ComboRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f ComboRow) MinWidth(minWidth int) ComboRow {
	return func() *adw.ComboRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f ComboRow) Padding(padding int) ComboRow {
	return func() *adw.ComboRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f ComboRow) PaddingBottom(padding int) ComboRow {
	return func() *adw.ComboRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f ComboRow) PaddingEnd(padding int) ComboRow {
	return func() *adw.ComboRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f ComboRow) PaddingStart(padding int) ComboRow {
	return func() *adw.ComboRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f ComboRow) PaddingTop(padding int) ComboRow {
	return func() *adw.ComboRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f ComboRow) VPadding(padding int) ComboRow {
	return func() *adw.ComboRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f ComboRow) BindVisible(state *state.State[bool]) ComboRow {
	return func() *adw.ComboRow {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *adw.ComboRow) {
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

func (f ComboRow) BindHMargin(state *state.State[int32]) ComboRow {
	return func() *adw.ComboRow {
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

func (f ComboRow) BindMargin(state *state.State[int32]) ComboRow {
	return func() *adw.ComboRow {
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

func (f ComboRow) BindMarginBottom(state *state.State[int32]) ComboRow {
	return func() *adw.ComboRow {
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

func (f ComboRow) BindMarginEnd(state *state.State[int32]) ComboRow {
	return func() *adw.ComboRow {
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

func (f ComboRow) BindMarginStart(state *state.State[int32]) ComboRow {
	return func() *adw.ComboRow {
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

func (f ComboRow) BindMarginTop(state *state.State[int32]) ComboRow {
	return func() *adw.ComboRow {
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

func (f ComboRow) BindSensitive(state *state.State[bool]) ComboRow {
	return func() *adw.ComboRow {
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
