package adw

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type ExpanderRow func() *adw.ExpanderRow

func (f ExpanderRow) AddController(controller *gtk.EventController) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f ExpanderRow) ConnectConstruct(cb func(*adw.ExpanderRow)) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f ExpanderRow) ConnectDestroy(cb func(gtk.Widget)) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f ExpanderRow) ConnectHide(cb func(gtk.Widget)) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f ExpanderRow) ConnectMap(cb func(gtk.Widget)) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f ExpanderRow) ConnectRealize(cb func(gtk.Widget)) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f ExpanderRow) ConnectShow(cb func(gtk.Widget)) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f ExpanderRow) ConnectUnmap(cb func(gtk.Widget)) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f ExpanderRow) ConnectUnrealize(cb func(gtk.Widget)) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f ExpanderRow) Controller(controller *gtk.EventController) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f ExpanderRow) Focusable(focusable bool) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f ExpanderRow) FocusOnClick(focusOnClick bool) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f ExpanderRow) HAlign(align gtk.Align) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f ExpanderRow) HExpand(expand bool) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f ExpanderRow) HMargin(horizontal int32) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f ExpanderRow) Margin(margin int32) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f ExpanderRow) MarginBottom(bottom int32) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f ExpanderRow) MarginEnd(end int32) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f ExpanderRow) MarginStart(start int32) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f ExpanderRow) MarginTop(top int32) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f ExpanderRow) Opacity(opacity float64) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f ExpanderRow) Overflow(overflow gtk.Overflow) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f ExpanderRow) Sensitive(sensitive bool) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f ExpanderRow) SizeRequest(width, height int32) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f ExpanderRow) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f ExpanderRow) VAlign(align gtk.Align) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f ExpanderRow) VExpand(expand bool) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f ExpanderRow) Visible(visible bool) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f ExpanderRow) VMargin(vertical int32) ExpanderRow {
	return func() *adw.ExpanderRow {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f ExpanderRow) Background(color string) ExpanderRow {
	return func() *adw.ExpanderRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f ExpanderRow) CornerRadius(radius int) ExpanderRow {
	return func() *adw.ExpanderRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f ExpanderRow) CSS(css string) ExpanderRow {
	return func() *adw.ExpanderRow {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f ExpanderRow) BindCSSClass(state *state.State[string]) ExpanderRow {
	return func() *adw.ExpanderRow {
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

func (f ExpanderRow) WithCSSClass(className string) ExpanderRow {
	return func() *adw.ExpanderRow {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f ExpanderRow) CSSWithCallback(cb func(elementName string) string) ExpanderRow {
	return func() *adw.ExpanderRow {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *adw.ExpanderRow) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f ExpanderRow) HPadding(padding int) ExpanderRow {
	return func() *adw.ExpanderRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f ExpanderRow) MinHeight(minHeight int) ExpanderRow {
	return func() *adw.ExpanderRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f ExpanderRow) MinWidth(minWidth int) ExpanderRow {
	return func() *adw.ExpanderRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f ExpanderRow) Padding(padding int) ExpanderRow {
	return func() *adw.ExpanderRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f ExpanderRow) PaddingBottom(padding int) ExpanderRow {
	return func() *adw.ExpanderRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f ExpanderRow) PaddingEnd(padding int) ExpanderRow {
	return func() *adw.ExpanderRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f ExpanderRow) PaddingStart(padding int) ExpanderRow {
	return func() *adw.ExpanderRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f ExpanderRow) PaddingTop(padding int) ExpanderRow {
	return func() *adw.ExpanderRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f ExpanderRow) VPadding(padding int) ExpanderRow {
	return func() *adw.ExpanderRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f ExpanderRow) BindVisible(state *state.State[bool]) ExpanderRow {
	return func() *adw.ExpanderRow {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *adw.ExpanderRow) {
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

func (f ExpanderRow) BindHMargin(state *state.State[int32]) ExpanderRow {
	return func() *adw.ExpanderRow {
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

func (f ExpanderRow) BindMargin(state *state.State[int32]) ExpanderRow {
	return func() *adw.ExpanderRow {
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

func (f ExpanderRow) BindMarginBottom(state *state.State[int32]) ExpanderRow {
	return func() *adw.ExpanderRow {
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

func (f ExpanderRow) BindMarginEnd(state *state.State[int32]) ExpanderRow {
	return func() *adw.ExpanderRow {
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

func (f ExpanderRow) BindMarginStart(state *state.State[int32]) ExpanderRow {
	return func() *adw.ExpanderRow {
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

func (f ExpanderRow) BindMarginTop(state *state.State[int32]) ExpanderRow {
	return func() *adw.ExpanderRow {
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

func (f ExpanderRow) BindSensitive(state *state.State[bool]) ExpanderRow {
	return func() *adw.ExpanderRow {
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
