package adw

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type EntryRow func() *adw.EntryRow

func (f EntryRow) AddController(controller *gtk.EventController) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f EntryRow) ConnectConstruct(cb func(*adw.EntryRow)) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f EntryRow) ConnectDestroy(cb func(gtk.Widget)) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f EntryRow) ConnectHide(cb func(gtk.Widget)) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f EntryRow) ConnectMap(cb func(gtk.Widget)) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f EntryRow) ConnectRealize(cb func(gtk.Widget)) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f EntryRow) ConnectShow(cb func(gtk.Widget)) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f EntryRow) ConnectUnmap(cb func(gtk.Widget)) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f EntryRow) ConnectUnrealize(cb func(gtk.Widget)) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f EntryRow) Controller(controller *gtk.EventController) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f EntryRow) Focusable(focusable bool) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f EntryRow) FocusOnClick(focusOnClick bool) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f EntryRow) HAlign(align gtk.Align) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f EntryRow) HExpand(expand bool) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f EntryRow) HMargin(horizontal int32) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f EntryRow) Margin(margin int32) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f EntryRow) MarginBottom(bottom int32) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f EntryRow) MarginEnd(end int32) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f EntryRow) MarginStart(start int32) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f EntryRow) MarginTop(top int32) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f EntryRow) Opacity(opacity float64) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f EntryRow) Overflow(overflow gtk.Overflow) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f EntryRow) Sensitive(sensitive bool) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f EntryRow) SizeRequest(width, height int32) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f EntryRow) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f EntryRow) VAlign(align gtk.Align) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f EntryRow) VExpand(expand bool) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f EntryRow) Visible(visible bool) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f EntryRow) VMargin(vertical int32) EntryRow {
	return func() *adw.EntryRow {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f EntryRow) Background(color string) EntryRow {
	return func() *adw.EntryRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f EntryRow) CornerRadius(radius int) EntryRow {
	return func() *adw.EntryRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f EntryRow) CSS(css string) EntryRow {
	return func() *adw.EntryRow {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f EntryRow) BindCSSClass(state *state.State[string]) EntryRow {
	return func() *adw.EntryRow {
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

func (f EntryRow) WithCSSClass(className string) EntryRow {
	return func() *adw.EntryRow {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f EntryRow) CSSWithCallback(cb func(elementName string) string) EntryRow {
	return func() *adw.EntryRow {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *adw.EntryRow) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f EntryRow) HPadding(padding int) EntryRow {
	return func() *adw.EntryRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f EntryRow) MinHeight(minHeight int) EntryRow {
	return func() *adw.EntryRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f EntryRow) MinWidth(minWidth int) EntryRow {
	return func() *adw.EntryRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f EntryRow) Padding(padding int) EntryRow {
	return func() *adw.EntryRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f EntryRow) PaddingBottom(padding int) EntryRow {
	return func() *adw.EntryRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f EntryRow) PaddingEnd(padding int) EntryRow {
	return func() *adw.EntryRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f EntryRow) PaddingStart(padding int) EntryRow {
	return func() *adw.EntryRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f EntryRow) PaddingTop(padding int) EntryRow {
	return func() *adw.EntryRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f EntryRow) VPadding(padding int) EntryRow {
	return func() *adw.EntryRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f EntryRow) BindVisible(state *state.State[bool]) EntryRow {
	return func() *adw.EntryRow {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *adw.EntryRow) {
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

func (f EntryRow) BindHMargin(state *state.State[int32]) EntryRow {
	return func() *adw.EntryRow {
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

func (f EntryRow) BindMargin(state *state.State[int32]) EntryRow {
	return func() *adw.EntryRow {
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

func (f EntryRow) BindMarginBottom(state *state.State[int32]) EntryRow {
	return func() *adw.EntryRow {
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

func (f EntryRow) BindMarginEnd(state *state.State[int32]) EntryRow {
	return func() *adw.EntryRow {
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

func (f EntryRow) BindMarginStart(state *state.State[int32]) EntryRow {
	return func() *adw.EntryRow {
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

func (f EntryRow) BindMarginTop(state *state.State[int32]) EntryRow {
	return func() *adw.EntryRow {
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

func (f EntryRow) BindSensitive(state *state.State[bool]) EntryRow {
	return func() *adw.EntryRow {
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
