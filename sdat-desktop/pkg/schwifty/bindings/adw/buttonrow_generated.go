package adw

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type ButtonRow func() *adw.ButtonRow

func (f ButtonRow) AddController(controller *gtk.EventController) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f ButtonRow) ConnectConstruct(cb func(*adw.ButtonRow)) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f ButtonRow) ConnectDestroy(cb func(gtk.Widget)) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f ButtonRow) ConnectHide(cb func(gtk.Widget)) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f ButtonRow) ConnectMap(cb func(gtk.Widget)) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f ButtonRow) ConnectRealize(cb func(gtk.Widget)) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f ButtonRow) ConnectShow(cb func(gtk.Widget)) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f ButtonRow) ConnectUnmap(cb func(gtk.Widget)) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f ButtonRow) ConnectUnrealize(cb func(gtk.Widget)) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f ButtonRow) Controller(controller *gtk.EventController) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f ButtonRow) Focusable(focusable bool) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f ButtonRow) FocusOnClick(focusOnClick bool) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f ButtonRow) HAlign(align gtk.Align) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f ButtonRow) HExpand(expand bool) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f ButtonRow) HMargin(horizontal int32) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f ButtonRow) Margin(margin int32) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f ButtonRow) MarginBottom(bottom int32) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f ButtonRow) MarginEnd(end int32) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f ButtonRow) MarginStart(start int32) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f ButtonRow) MarginTop(top int32) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f ButtonRow) Opacity(opacity float64) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f ButtonRow) Overflow(overflow gtk.Overflow) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f ButtonRow) Sensitive(sensitive bool) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f ButtonRow) SizeRequest(width, height int32) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f ButtonRow) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f ButtonRow) VAlign(align gtk.Align) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f ButtonRow) VExpand(expand bool) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f ButtonRow) Visible(visible bool) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f ButtonRow) VMargin(vertical int32) ButtonRow {
	return func() *adw.ButtonRow {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f ButtonRow) Background(color string) ButtonRow {
	return func() *adw.ButtonRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f ButtonRow) CornerRadius(radius int) ButtonRow {
	return func() *adw.ButtonRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f ButtonRow) CSS(css string) ButtonRow {
	return func() *adw.ButtonRow {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f ButtonRow) BindCSSClass(state *state.State[string]) ButtonRow {
	return func() *adw.ButtonRow {
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

func (f ButtonRow) WithCSSClass(className string) ButtonRow {
	return func() *adw.ButtonRow {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f ButtonRow) CSSWithCallback(cb func(elementName string) string) ButtonRow {
	return func() *adw.ButtonRow {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *adw.ButtonRow) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f ButtonRow) HPadding(padding int) ButtonRow {
	return func() *adw.ButtonRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f ButtonRow) MinHeight(minHeight int) ButtonRow {
	return func() *adw.ButtonRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f ButtonRow) MinWidth(minWidth int) ButtonRow {
	return func() *adw.ButtonRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f ButtonRow) Padding(padding int) ButtonRow {
	return func() *adw.ButtonRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f ButtonRow) PaddingBottom(padding int) ButtonRow {
	return func() *adw.ButtonRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f ButtonRow) PaddingEnd(padding int) ButtonRow {
	return func() *adw.ButtonRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f ButtonRow) PaddingStart(padding int) ButtonRow {
	return func() *adw.ButtonRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f ButtonRow) PaddingTop(padding int) ButtonRow {
	return func() *adw.ButtonRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f ButtonRow) VPadding(padding int) ButtonRow {
	return func() *adw.ButtonRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f ButtonRow) BindVisible(state *state.State[bool]) ButtonRow {
	return func() *adw.ButtonRow {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *adw.ButtonRow) {
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

func (f ButtonRow) BindHMargin(state *state.State[int32]) ButtonRow {
	return func() *adw.ButtonRow {
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

func (f ButtonRow) BindMargin(state *state.State[int32]) ButtonRow {
	return func() *adw.ButtonRow {
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

func (f ButtonRow) BindMarginBottom(state *state.State[int32]) ButtonRow {
	return func() *adw.ButtonRow {
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

func (f ButtonRow) BindMarginEnd(state *state.State[int32]) ButtonRow {
	return func() *adw.ButtonRow {
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

func (f ButtonRow) BindMarginStart(state *state.State[int32]) ButtonRow {
	return func() *adw.ButtonRow {
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

func (f ButtonRow) BindMarginTop(state *state.State[int32]) ButtonRow {
	return func() *adw.ButtonRow {
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

func (f ButtonRow) BindSensitive(state *state.State[bool]) ButtonRow {
	return func() *adw.ButtonRow {
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
