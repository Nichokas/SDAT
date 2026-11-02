package adw

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type WindowTitle func() *adw.WindowTitle

func (f WindowTitle) AddController(controller *gtk.EventController) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f WindowTitle) ConnectConstruct(cb func(*adw.WindowTitle)) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f WindowTitle) ConnectDestroy(cb func(gtk.Widget)) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f WindowTitle) ConnectHide(cb func(gtk.Widget)) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f WindowTitle) ConnectMap(cb func(gtk.Widget)) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f WindowTitle) ConnectRealize(cb func(gtk.Widget)) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f WindowTitle) ConnectShow(cb func(gtk.Widget)) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f WindowTitle) ConnectUnmap(cb func(gtk.Widget)) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f WindowTitle) ConnectUnrealize(cb func(gtk.Widget)) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f WindowTitle) Controller(controller *gtk.EventController) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f WindowTitle) Focusable(focusable bool) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f WindowTitle) FocusOnClick(focusOnClick bool) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f WindowTitle) HAlign(align gtk.Align) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f WindowTitle) HExpand(expand bool) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f WindowTitle) HMargin(horizontal int32) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f WindowTitle) Margin(margin int32) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f WindowTitle) MarginBottom(bottom int32) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f WindowTitle) MarginEnd(end int32) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f WindowTitle) MarginStart(start int32) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f WindowTitle) MarginTop(top int32) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f WindowTitle) Opacity(opacity float64) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f WindowTitle) Overflow(overflow gtk.Overflow) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f WindowTitle) Sensitive(sensitive bool) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f WindowTitle) SizeRequest(width, height int32) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f WindowTitle) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f WindowTitle) VAlign(align gtk.Align) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f WindowTitle) VExpand(expand bool) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f WindowTitle) Visible(visible bool) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f WindowTitle) VMargin(vertical int32) WindowTitle {
	return func() *adw.WindowTitle {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f WindowTitle) Background(color string) WindowTitle {
	return func() *adw.WindowTitle {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f WindowTitle) CornerRadius(radius int) WindowTitle {
	return func() *adw.WindowTitle {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f WindowTitle) CSS(css string) WindowTitle {
	return func() *adw.WindowTitle {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f WindowTitle) BindCSSClass(state *state.State[string]) WindowTitle {
	return func() *adw.WindowTitle {
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

func (f WindowTitle) WithCSSClass(className string) WindowTitle {
	return func() *adw.WindowTitle {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f WindowTitle) CSSWithCallback(cb func(elementName string) string) WindowTitle {
	return func() *adw.WindowTitle {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *adw.WindowTitle) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f WindowTitle) HPadding(padding int) WindowTitle {
	return func() *adw.WindowTitle {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f WindowTitle) MinHeight(minHeight int) WindowTitle {
	return func() *adw.WindowTitle {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f WindowTitle) MinWidth(minWidth int) WindowTitle {
	return func() *adw.WindowTitle {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f WindowTitle) Padding(padding int) WindowTitle {
	return func() *adw.WindowTitle {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f WindowTitle) PaddingBottom(padding int) WindowTitle {
	return func() *adw.WindowTitle {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f WindowTitle) PaddingEnd(padding int) WindowTitle {
	return func() *adw.WindowTitle {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f WindowTitle) PaddingStart(padding int) WindowTitle {
	return func() *adw.WindowTitle {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f WindowTitle) PaddingTop(padding int) WindowTitle {
	return func() *adw.WindowTitle {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f WindowTitle) VPadding(padding int) WindowTitle {
	return func() *adw.WindowTitle {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f WindowTitle) BindVisible(state *state.State[bool]) WindowTitle {
	return func() *adw.WindowTitle {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *adw.WindowTitle) {
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

func (f WindowTitle) BindHMargin(state *state.State[int32]) WindowTitle {
	return func() *adw.WindowTitle {
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

func (f WindowTitle) BindMargin(state *state.State[int32]) WindowTitle {
	return func() *adw.WindowTitle {
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

func (f WindowTitle) BindMarginBottom(state *state.State[int32]) WindowTitle {
	return func() *adw.WindowTitle {
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

func (f WindowTitle) BindMarginEnd(state *state.State[int32]) WindowTitle {
	return func() *adw.WindowTitle {
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

func (f WindowTitle) BindMarginStart(state *state.State[int32]) WindowTitle {
	return func() *adw.WindowTitle {
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

func (f WindowTitle) BindMarginTop(state *state.State[int32]) WindowTitle {
	return func() *adw.WindowTitle {
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

func (f WindowTitle) BindSensitive(state *state.State[bool]) WindowTitle {
	return func() *adw.WindowTitle {
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
