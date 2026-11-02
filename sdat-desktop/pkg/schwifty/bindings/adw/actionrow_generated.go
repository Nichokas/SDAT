package adw

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type ActionRow func() *adw.ActionRow

func (f ActionRow) AddController(controller *gtk.EventController) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f ActionRow) ConnectConstruct(cb func(*adw.ActionRow)) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f ActionRow) ConnectDestroy(cb func(gtk.Widget)) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f ActionRow) ConnectHide(cb func(gtk.Widget)) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f ActionRow) ConnectMap(cb func(gtk.Widget)) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f ActionRow) ConnectRealize(cb func(gtk.Widget)) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f ActionRow) ConnectShow(cb func(gtk.Widget)) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f ActionRow) ConnectUnmap(cb func(gtk.Widget)) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f ActionRow) ConnectUnrealize(cb func(gtk.Widget)) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f ActionRow) Controller(controller *gtk.EventController) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f ActionRow) Focusable(focusable bool) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f ActionRow) FocusOnClick(focusOnClick bool) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f ActionRow) HAlign(align gtk.Align) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f ActionRow) HExpand(expand bool) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f ActionRow) HMargin(horizontal int32) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f ActionRow) Margin(margin int32) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f ActionRow) MarginBottom(bottom int32) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f ActionRow) MarginEnd(end int32) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f ActionRow) MarginStart(start int32) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f ActionRow) MarginTop(top int32) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f ActionRow) Opacity(opacity float64) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f ActionRow) Overflow(overflow gtk.Overflow) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f ActionRow) Sensitive(sensitive bool) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f ActionRow) SizeRequest(width, height int32) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f ActionRow) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f ActionRow) VAlign(align gtk.Align) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f ActionRow) VExpand(expand bool) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f ActionRow) Visible(visible bool) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f ActionRow) VMargin(vertical int32) ActionRow {
	return func() *adw.ActionRow {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f ActionRow) Background(color string) ActionRow {
	return func() *adw.ActionRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f ActionRow) CornerRadius(radius int) ActionRow {
	return func() *adw.ActionRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f ActionRow) CSS(css string) ActionRow {
	return func() *adw.ActionRow {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f ActionRow) BindCSSClass(state *state.State[string]) ActionRow {
	return func() *adw.ActionRow {
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

func (f ActionRow) WithCSSClass(className string) ActionRow {
	return func() *adw.ActionRow {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f ActionRow) CSSWithCallback(cb func(elementName string) string) ActionRow {
	return func() *adw.ActionRow {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *adw.ActionRow) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f ActionRow) HPadding(padding int) ActionRow {
	return func() *adw.ActionRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f ActionRow) MinHeight(minHeight int) ActionRow {
	return func() *adw.ActionRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f ActionRow) MinWidth(minWidth int) ActionRow {
	return func() *adw.ActionRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f ActionRow) Padding(padding int) ActionRow {
	return func() *adw.ActionRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f ActionRow) PaddingBottom(padding int) ActionRow {
	return func() *adw.ActionRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f ActionRow) PaddingEnd(padding int) ActionRow {
	return func() *adw.ActionRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f ActionRow) PaddingStart(padding int) ActionRow {
	return func() *adw.ActionRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f ActionRow) PaddingTop(padding int) ActionRow {
	return func() *adw.ActionRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f ActionRow) VPadding(padding int) ActionRow {
	return func() *adw.ActionRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f ActionRow) BindVisible(state *state.State[bool]) ActionRow {
	return func() *adw.ActionRow {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *adw.ActionRow) {
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

func (f ActionRow) BindHMargin(state *state.State[int32]) ActionRow {
	return func() *adw.ActionRow {
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

func (f ActionRow) BindMargin(state *state.State[int32]) ActionRow {
	return func() *adw.ActionRow {
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

func (f ActionRow) BindMarginBottom(state *state.State[int32]) ActionRow {
	return func() *adw.ActionRow {
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

func (f ActionRow) BindMarginEnd(state *state.State[int32]) ActionRow {
	return func() *adw.ActionRow {
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

func (f ActionRow) BindMarginStart(state *state.State[int32]) ActionRow {
	return func() *adw.ActionRow {
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

func (f ActionRow) BindMarginTop(state *state.State[int32]) ActionRow {
	return func() *adw.ActionRow {
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

func (f ActionRow) BindSensitive(state *state.State[bool]) ActionRow {
	return func() *adw.ActionRow {
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
