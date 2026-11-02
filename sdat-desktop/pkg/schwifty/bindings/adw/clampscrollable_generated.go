package adw

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type ClampScrollable func() *adw.ClampScrollable

func (f ClampScrollable) AddController(controller *gtk.EventController) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f ClampScrollable) ConnectConstruct(cb func(*adw.ClampScrollable)) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f ClampScrollable) ConnectDestroy(cb func(gtk.Widget)) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f ClampScrollable) ConnectHide(cb func(gtk.Widget)) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f ClampScrollable) ConnectMap(cb func(gtk.Widget)) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f ClampScrollable) ConnectRealize(cb func(gtk.Widget)) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f ClampScrollable) ConnectShow(cb func(gtk.Widget)) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f ClampScrollable) ConnectUnmap(cb func(gtk.Widget)) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f ClampScrollable) ConnectUnrealize(cb func(gtk.Widget)) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f ClampScrollable) Controller(controller *gtk.EventController) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f ClampScrollable) Focusable(focusable bool) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f ClampScrollable) FocusOnClick(focusOnClick bool) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f ClampScrollable) HAlign(align gtk.Align) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f ClampScrollable) HExpand(expand bool) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f ClampScrollable) HMargin(horizontal int32) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f ClampScrollable) Margin(margin int32) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f ClampScrollable) MarginBottom(bottom int32) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f ClampScrollable) MarginEnd(end int32) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f ClampScrollable) MarginStart(start int32) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f ClampScrollable) MarginTop(top int32) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f ClampScrollable) Opacity(opacity float64) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f ClampScrollable) Overflow(overflow gtk.Overflow) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f ClampScrollable) Sensitive(sensitive bool) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f ClampScrollable) SizeRequest(width, height int32) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f ClampScrollable) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f ClampScrollable) VAlign(align gtk.Align) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f ClampScrollable) VExpand(expand bool) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f ClampScrollable) Visible(visible bool) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f ClampScrollable) VMargin(vertical int32) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f ClampScrollable) Background(color string) ClampScrollable {
	return func() *adw.ClampScrollable {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f ClampScrollable) CornerRadius(radius int) ClampScrollable {
	return func() *adw.ClampScrollable {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f ClampScrollable) CSS(css string) ClampScrollable {
	return func() *adw.ClampScrollable {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f ClampScrollable) BindCSSClass(state *state.State[string]) ClampScrollable {
	return func() *adw.ClampScrollable {
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

func (f ClampScrollable) WithCSSClass(className string) ClampScrollable {
	return func() *adw.ClampScrollable {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f ClampScrollable) CSSWithCallback(cb func(elementName string) string) ClampScrollable {
	return func() *adw.ClampScrollable {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *adw.ClampScrollable) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f ClampScrollable) HPadding(padding int) ClampScrollable {
	return func() *adw.ClampScrollable {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f ClampScrollable) MinHeight(minHeight int) ClampScrollable {
	return func() *adw.ClampScrollable {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f ClampScrollable) MinWidth(minWidth int) ClampScrollable {
	return func() *adw.ClampScrollable {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f ClampScrollable) Padding(padding int) ClampScrollable {
	return func() *adw.ClampScrollable {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f ClampScrollable) PaddingBottom(padding int) ClampScrollable {
	return func() *adw.ClampScrollable {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f ClampScrollable) PaddingEnd(padding int) ClampScrollable {
	return func() *adw.ClampScrollable {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f ClampScrollable) PaddingStart(padding int) ClampScrollable {
	return func() *adw.ClampScrollable {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f ClampScrollable) PaddingTop(padding int) ClampScrollable {
	return func() *adw.ClampScrollable {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f ClampScrollable) VPadding(padding int) ClampScrollable {
	return func() *adw.ClampScrollable {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f ClampScrollable) BindVisible(state *state.State[bool]) ClampScrollable {
	return func() *adw.ClampScrollable {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *adw.ClampScrollable) {
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

func (f ClampScrollable) BindHMargin(state *state.State[int32]) ClampScrollable {
	return func() *adw.ClampScrollable {
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

func (f ClampScrollable) BindMargin(state *state.State[int32]) ClampScrollable {
	return func() *adw.ClampScrollable {
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

func (f ClampScrollable) BindMarginBottom(state *state.State[int32]) ClampScrollable {
	return func() *adw.ClampScrollable {
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

func (f ClampScrollable) BindMarginEnd(state *state.State[int32]) ClampScrollable {
	return func() *adw.ClampScrollable {
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

func (f ClampScrollable) BindMarginStart(state *state.State[int32]) ClampScrollable {
	return func() *adw.ClampScrollable {
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

func (f ClampScrollable) BindMarginTop(state *state.State[int32]) ClampScrollable {
	return func() *adw.ClampScrollable {
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

func (f ClampScrollable) BindSensitive(state *state.State[bool]) ClampScrollable {
	return func() *adw.ClampScrollable {
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
