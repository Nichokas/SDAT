package adw

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type HeaderBar func() *adw.HeaderBar

func (f HeaderBar) AddController(controller *gtk.EventController) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f HeaderBar) ConnectConstruct(cb func(*adw.HeaderBar)) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f HeaderBar) ConnectDestroy(cb func(gtk.Widget)) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f HeaderBar) ConnectHide(cb func(gtk.Widget)) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f HeaderBar) ConnectMap(cb func(gtk.Widget)) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f HeaderBar) ConnectRealize(cb func(gtk.Widget)) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f HeaderBar) ConnectShow(cb func(gtk.Widget)) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f HeaderBar) ConnectUnmap(cb func(gtk.Widget)) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f HeaderBar) ConnectUnrealize(cb func(gtk.Widget)) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f HeaderBar) Controller(controller *gtk.EventController) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f HeaderBar) Focusable(focusable bool) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f HeaderBar) FocusOnClick(focusOnClick bool) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f HeaderBar) HAlign(align gtk.Align) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f HeaderBar) HExpand(expand bool) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f HeaderBar) HMargin(horizontal int32) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f HeaderBar) Margin(margin int32) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f HeaderBar) MarginBottom(bottom int32) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f HeaderBar) MarginEnd(end int32) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f HeaderBar) MarginStart(start int32) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f HeaderBar) MarginTop(top int32) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f HeaderBar) Opacity(opacity float64) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f HeaderBar) Overflow(overflow gtk.Overflow) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f HeaderBar) Sensitive(sensitive bool) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f HeaderBar) SizeRequest(width, height int32) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f HeaderBar) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f HeaderBar) VAlign(align gtk.Align) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f HeaderBar) VExpand(expand bool) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f HeaderBar) Visible(visible bool) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f HeaderBar) VMargin(vertical int32) HeaderBar {
	return func() *adw.HeaderBar {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f HeaderBar) Background(color string) HeaderBar {
	return func() *adw.HeaderBar {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f HeaderBar) CornerRadius(radius int) HeaderBar {
	return func() *adw.HeaderBar {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f HeaderBar) CSS(css string) HeaderBar {
	return func() *adw.HeaderBar {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f HeaderBar) BindCSSClass(state *state.State[string]) HeaderBar {
	return func() *adw.HeaderBar {
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

func (f HeaderBar) WithCSSClass(className string) HeaderBar {
	return func() *adw.HeaderBar {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f HeaderBar) CSSWithCallback(cb func(elementName string) string) HeaderBar {
	return func() *adw.HeaderBar {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *adw.HeaderBar) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f HeaderBar) HPadding(padding int) HeaderBar {
	return func() *adw.HeaderBar {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f HeaderBar) MinHeight(minHeight int) HeaderBar {
	return func() *adw.HeaderBar {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f HeaderBar) MinWidth(minWidth int) HeaderBar {
	return func() *adw.HeaderBar {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f HeaderBar) Padding(padding int) HeaderBar {
	return func() *adw.HeaderBar {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f HeaderBar) PaddingBottom(padding int) HeaderBar {
	return func() *adw.HeaderBar {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f HeaderBar) PaddingEnd(padding int) HeaderBar {
	return func() *adw.HeaderBar {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f HeaderBar) PaddingStart(padding int) HeaderBar {
	return func() *adw.HeaderBar {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f HeaderBar) PaddingTop(padding int) HeaderBar {
	return func() *adw.HeaderBar {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f HeaderBar) VPadding(padding int) HeaderBar {
	return func() *adw.HeaderBar {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f HeaderBar) BindVisible(state *state.State[bool]) HeaderBar {
	return func() *adw.HeaderBar {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *adw.HeaderBar) {
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

func (f HeaderBar) BindHMargin(state *state.State[int32]) HeaderBar {
	return func() *adw.HeaderBar {
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

func (f HeaderBar) BindMargin(state *state.State[int32]) HeaderBar {
	return func() *adw.HeaderBar {
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

func (f HeaderBar) BindMarginBottom(state *state.State[int32]) HeaderBar {
	return func() *adw.HeaderBar {
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

func (f HeaderBar) BindMarginEnd(state *state.State[int32]) HeaderBar {
	return func() *adw.HeaderBar {
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

func (f HeaderBar) BindMarginStart(state *state.State[int32]) HeaderBar {
	return func() *adw.HeaderBar {
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

func (f HeaderBar) BindMarginTop(state *state.State[int32]) HeaderBar {
	return func() *adw.HeaderBar {
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

func (f HeaderBar) BindSensitive(state *state.State[bool]) HeaderBar {
	return func() *adw.HeaderBar {
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
