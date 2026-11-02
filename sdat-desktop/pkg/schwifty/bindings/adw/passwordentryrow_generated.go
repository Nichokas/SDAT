package adw

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type PasswordEntryRow func() *adw.PasswordEntryRow

func (f PasswordEntryRow) AddController(controller *gtk.EventController) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f PasswordEntryRow) ConnectConstruct(cb func(*adw.PasswordEntryRow)) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f PasswordEntryRow) ConnectDestroy(cb func(gtk.Widget)) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f PasswordEntryRow) ConnectHide(cb func(gtk.Widget)) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f PasswordEntryRow) ConnectMap(cb func(gtk.Widget)) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f PasswordEntryRow) ConnectRealize(cb func(gtk.Widget)) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f PasswordEntryRow) ConnectShow(cb func(gtk.Widget)) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f PasswordEntryRow) ConnectUnmap(cb func(gtk.Widget)) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f PasswordEntryRow) ConnectUnrealize(cb func(gtk.Widget)) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f PasswordEntryRow) Controller(controller *gtk.EventController) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f PasswordEntryRow) Focusable(focusable bool) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f PasswordEntryRow) FocusOnClick(focusOnClick bool) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f PasswordEntryRow) HAlign(align gtk.Align) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f PasswordEntryRow) HExpand(expand bool) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f PasswordEntryRow) HMargin(horizontal int32) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f PasswordEntryRow) Margin(margin int32) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f PasswordEntryRow) MarginBottom(bottom int32) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f PasswordEntryRow) MarginEnd(end int32) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f PasswordEntryRow) MarginStart(start int32) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f PasswordEntryRow) MarginTop(top int32) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f PasswordEntryRow) Opacity(opacity float64) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f PasswordEntryRow) Overflow(overflow gtk.Overflow) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f PasswordEntryRow) Sensitive(sensitive bool) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f PasswordEntryRow) SizeRequest(width, height int32) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f PasswordEntryRow) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f PasswordEntryRow) VAlign(align gtk.Align) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f PasswordEntryRow) VExpand(expand bool) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f PasswordEntryRow) Visible(visible bool) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f PasswordEntryRow) VMargin(vertical int32) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f PasswordEntryRow) Background(color string) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f PasswordEntryRow) CornerRadius(radius int) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f PasswordEntryRow) CSS(css string) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f PasswordEntryRow) BindCSSClass(state *state.State[string]) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
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

func (f PasswordEntryRow) WithCSSClass(className string) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f PasswordEntryRow) CSSWithCallback(cb func(elementName string) string) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *adw.PasswordEntryRow) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f PasswordEntryRow) HPadding(padding int) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f PasswordEntryRow) MinHeight(minHeight int) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f PasswordEntryRow) MinWidth(minWidth int) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f PasswordEntryRow) Padding(padding int) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f PasswordEntryRow) PaddingBottom(padding int) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f PasswordEntryRow) PaddingEnd(padding int) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f PasswordEntryRow) PaddingStart(padding int) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f PasswordEntryRow) PaddingTop(padding int) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f PasswordEntryRow) VPadding(padding int) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f PasswordEntryRow) BindVisible(state *state.State[bool]) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *adw.PasswordEntryRow) {
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

func (f PasswordEntryRow) BindHMargin(state *state.State[int32]) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
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

func (f PasswordEntryRow) BindMargin(state *state.State[int32]) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
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

func (f PasswordEntryRow) BindMarginBottom(state *state.State[int32]) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
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

func (f PasswordEntryRow) BindMarginEnd(state *state.State[int32]) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
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

func (f PasswordEntryRow) BindMarginStart(state *state.State[int32]) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
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

func (f PasswordEntryRow) BindMarginTop(state *state.State[int32]) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
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

func (f PasswordEntryRow) BindSensitive(state *state.State[bool]) PasswordEntryRow {
	return func() *adw.PasswordEntryRow {
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
