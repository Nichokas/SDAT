package gtk

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type Box func() *gtk.Box

func (f Box) AddController(controller *gtk.EventController) Box {
	return func() *gtk.Box {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f Box) ConnectConstruct(cb func(*gtk.Box)) Box {
	return func() *gtk.Box {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f Box) ConnectDestroy(cb func(gtk.Widget)) Box {
	return func() *gtk.Box {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f Box) ConnectHide(cb func(gtk.Widget)) Box {
	return func() *gtk.Box {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f Box) ConnectMap(cb func(gtk.Widget)) Box {
	return func() *gtk.Box {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f Box) ConnectRealize(cb func(gtk.Widget)) Box {
	return func() *gtk.Box {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f Box) ConnectShow(cb func(gtk.Widget)) Box {
	return func() *gtk.Box {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f Box) ConnectUnmap(cb func(gtk.Widget)) Box {
	return func() *gtk.Box {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f Box) ConnectUnrealize(cb func(gtk.Widget)) Box {
	return func() *gtk.Box {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f Box) Controller(controller *gtk.EventController) Box {
	return func() *gtk.Box {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f Box) Focusable(focusable bool) Box {
	return func() *gtk.Box {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f Box) FocusOnClick(focusOnClick bool) Box {
	return func() *gtk.Box {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f Box) HAlign(align gtk.Align) Box {
	return func() *gtk.Box {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f Box) HExpand(expand bool) Box {
	return func() *gtk.Box {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f Box) HMargin(horizontal int32) Box {
	return func() *gtk.Box {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f Box) Margin(margin int32) Box {
	return func() *gtk.Box {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f Box) MarginBottom(bottom int32) Box {
	return func() *gtk.Box {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f Box) MarginEnd(end int32) Box {
	return func() *gtk.Box {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f Box) MarginStart(start int32) Box {
	return func() *gtk.Box {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f Box) MarginTop(top int32) Box {
	return func() *gtk.Box {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f Box) Opacity(opacity float64) Box {
	return func() *gtk.Box {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f Box) Overflow(overflow gtk.Overflow) Box {
	return func() *gtk.Box {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f Box) Sensitive(sensitive bool) Box {
	return func() *gtk.Box {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f Box) SizeRequest(width, height int32) Box {
	return func() *gtk.Box {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f Box) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f Box) VAlign(align gtk.Align) Box {
	return func() *gtk.Box {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f Box) VExpand(expand bool) Box {
	return func() *gtk.Box {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f Box) Visible(visible bool) Box {
	return func() *gtk.Box {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f Box) VMargin(vertical int32) Box {
	return func() *gtk.Box {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f Box) Background(color string) Box {
	return func() *gtk.Box {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f Box) CornerRadius(radius int) Box {
	return func() *gtk.Box {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f Box) CSS(css string) Box {
	return func() *gtk.Box {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f Box) BindCSSClass(state *state.State[string]) Box {
	return func() *gtk.Box {
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

func (f Box) WithCSSClass(className string) Box {
	return func() *gtk.Box {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f Box) CSSWithCallback(cb func(elementName string) string) Box {
	return func() *gtk.Box {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *gtk.Box) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f Box) HPadding(padding int) Box {
	return func() *gtk.Box {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f Box) MinHeight(minHeight int) Box {
	return func() *gtk.Box {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f Box) MinWidth(minWidth int) Box {
	return func() *gtk.Box {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f Box) Padding(padding int) Box {
	return func() *gtk.Box {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f Box) PaddingBottom(padding int) Box {
	return func() *gtk.Box {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f Box) PaddingEnd(padding int) Box {
	return func() *gtk.Box {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f Box) PaddingStart(padding int) Box {
	return func() *gtk.Box {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f Box) PaddingTop(padding int) Box {
	return func() *gtk.Box {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f Box) VPadding(padding int) Box {
	return func() *gtk.Box {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f Box) BindVisible(state *state.State[bool]) Box {
	return func() *gtk.Box {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *gtk.Box) {
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

func (f Box) BindHMargin(state *state.State[int32]) Box {
	return func() *gtk.Box {
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

func (f Box) BindMargin(state *state.State[int32]) Box {
	return func() *gtk.Box {
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

func (f Box) BindMarginBottom(state *state.State[int32]) Box {
	return func() *gtk.Box {
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

func (f Box) BindMarginEnd(state *state.State[int32]) Box {
	return func() *gtk.Box {
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

func (f Box) BindMarginStart(state *state.State[int32]) Box {
	return func() *gtk.Box {
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

func (f Box) BindMarginTop(state *state.State[int32]) Box {
	return func() *gtk.Box {
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

func (f Box) BindSensitive(state *state.State[bool]) Box {
	return func() *gtk.Box {
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
