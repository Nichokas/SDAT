package adw

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type SwitchRow func() *adw.SwitchRow

func (f SwitchRow) AddController(controller *gtk.EventController) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f SwitchRow) ConnectConstruct(cb func(*adw.SwitchRow)) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f SwitchRow) ConnectDestroy(cb func(gtk.Widget)) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f SwitchRow) ConnectHide(cb func(gtk.Widget)) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f SwitchRow) ConnectMap(cb func(gtk.Widget)) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f SwitchRow) ConnectRealize(cb func(gtk.Widget)) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f SwitchRow) ConnectShow(cb func(gtk.Widget)) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f SwitchRow) ConnectUnmap(cb func(gtk.Widget)) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f SwitchRow) ConnectUnrealize(cb func(gtk.Widget)) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f SwitchRow) Controller(controller *gtk.EventController) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f SwitchRow) Focusable(focusable bool) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f SwitchRow) FocusOnClick(focusOnClick bool) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f SwitchRow) HAlign(align gtk.Align) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f SwitchRow) HExpand(expand bool) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f SwitchRow) HMargin(horizontal int32) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f SwitchRow) Margin(margin int32) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f SwitchRow) MarginBottom(bottom int32) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f SwitchRow) MarginEnd(end int32) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f SwitchRow) MarginStart(start int32) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f SwitchRow) MarginTop(top int32) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f SwitchRow) Opacity(opacity float64) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f SwitchRow) Overflow(overflow gtk.Overflow) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f SwitchRow) Sensitive(sensitive bool) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f SwitchRow) SizeRequest(width, height int32) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f SwitchRow) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f SwitchRow) VAlign(align gtk.Align) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f SwitchRow) VExpand(expand bool) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f SwitchRow) Visible(visible bool) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f SwitchRow) VMargin(vertical int32) SwitchRow {
	return func() *adw.SwitchRow {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f SwitchRow) Background(color string) SwitchRow {
	return func() *adw.SwitchRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f SwitchRow) CornerRadius(radius int) SwitchRow {
	return func() *adw.SwitchRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f SwitchRow) CSS(css string) SwitchRow {
	return func() *adw.SwitchRow {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f SwitchRow) BindCSSClass(state *state.State[string]) SwitchRow {
	return func() *adw.SwitchRow {
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

func (f SwitchRow) WithCSSClass(className string) SwitchRow {
	return func() *adw.SwitchRow {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f SwitchRow) CSSWithCallback(cb func(elementName string) string) SwitchRow {
	return func() *adw.SwitchRow {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *adw.SwitchRow) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f SwitchRow) HPadding(padding int) SwitchRow {
	return func() *adw.SwitchRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f SwitchRow) MinHeight(minHeight int) SwitchRow {
	return func() *adw.SwitchRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f SwitchRow) MinWidth(minWidth int) SwitchRow {
	return func() *adw.SwitchRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f SwitchRow) Padding(padding int) SwitchRow {
	return func() *adw.SwitchRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f SwitchRow) PaddingBottom(padding int) SwitchRow {
	return func() *adw.SwitchRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f SwitchRow) PaddingEnd(padding int) SwitchRow {
	return func() *adw.SwitchRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f SwitchRow) PaddingStart(padding int) SwitchRow {
	return func() *adw.SwitchRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f SwitchRow) PaddingTop(padding int) SwitchRow {
	return func() *adw.SwitchRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f SwitchRow) VPadding(padding int) SwitchRow {
	return func() *adw.SwitchRow {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f SwitchRow) BindVisible(state *state.State[bool]) SwitchRow {
	return func() *adw.SwitchRow {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *adw.SwitchRow) {
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

func (f SwitchRow) BindHMargin(state *state.State[int32]) SwitchRow {
	return func() *adw.SwitchRow {
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

func (f SwitchRow) BindMargin(state *state.State[int32]) SwitchRow {
	return func() *adw.SwitchRow {
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

func (f SwitchRow) BindMarginBottom(state *state.State[int32]) SwitchRow {
	return func() *adw.SwitchRow {
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

func (f SwitchRow) BindMarginEnd(state *state.State[int32]) SwitchRow {
	return func() *adw.SwitchRow {
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

func (f SwitchRow) BindMarginStart(state *state.State[int32]) SwitchRow {
	return func() *adw.SwitchRow {
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

func (f SwitchRow) BindMarginTop(state *state.State[int32]) SwitchRow {
	return func() *adw.SwitchRow {
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

func (f SwitchRow) BindSensitive(state *state.State[bool]) SwitchRow {
	return func() *adw.SwitchRow {
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
