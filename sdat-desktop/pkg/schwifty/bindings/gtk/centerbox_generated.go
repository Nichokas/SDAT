package gtk

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type CenterBox func() *gtk.CenterBox

func (f CenterBox) AddController(controller *gtk.EventController) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f CenterBox) ConnectConstruct(cb func(*gtk.CenterBox)) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f CenterBox) ConnectDestroy(cb func(gtk.Widget)) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f CenterBox) ConnectHide(cb func(gtk.Widget)) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f CenterBox) ConnectMap(cb func(gtk.Widget)) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f CenterBox) ConnectRealize(cb func(gtk.Widget)) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f CenterBox) ConnectShow(cb func(gtk.Widget)) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f CenterBox) ConnectUnmap(cb func(gtk.Widget)) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f CenterBox) ConnectUnrealize(cb func(gtk.Widget)) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f CenterBox) Controller(controller *gtk.EventController) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f CenterBox) Focusable(focusable bool) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f CenterBox) FocusOnClick(focusOnClick bool) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f CenterBox) HAlign(align gtk.Align) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f CenterBox) HExpand(expand bool) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f CenterBox) HMargin(horizontal int32) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f CenterBox) Margin(margin int32) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f CenterBox) MarginBottom(bottom int32) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f CenterBox) MarginEnd(end int32) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f CenterBox) MarginStart(start int32) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f CenterBox) MarginTop(top int32) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f CenterBox) Opacity(opacity float64) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f CenterBox) Overflow(overflow gtk.Overflow) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f CenterBox) Sensitive(sensitive bool) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f CenterBox) SizeRequest(width, height int32) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f CenterBox) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f CenterBox) VAlign(align gtk.Align) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f CenterBox) VExpand(expand bool) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f CenterBox) Visible(visible bool) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f CenterBox) VMargin(vertical int32) CenterBox {
	return func() *gtk.CenterBox {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f CenterBox) Background(color string) CenterBox {
	return func() *gtk.CenterBox {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f CenterBox) CornerRadius(radius int) CenterBox {
	return func() *gtk.CenterBox {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f CenterBox) CSS(css string) CenterBox {
	return func() *gtk.CenterBox {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f CenterBox) BindCSSClass(state *state.State[string]) CenterBox {
	return func() *gtk.CenterBox {
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

func (f CenterBox) WithCSSClass(className string) CenterBox {
	return func() *gtk.CenterBox {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f CenterBox) CSSWithCallback(cb func(elementName string) string) CenterBox {
	return func() *gtk.CenterBox {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *gtk.CenterBox) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f CenterBox) HPadding(padding int) CenterBox {
	return func() *gtk.CenterBox {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f CenterBox) MinHeight(minHeight int) CenterBox {
	return func() *gtk.CenterBox {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f CenterBox) MinWidth(minWidth int) CenterBox {
	return func() *gtk.CenterBox {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f CenterBox) Padding(padding int) CenterBox {
	return func() *gtk.CenterBox {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f CenterBox) PaddingBottom(padding int) CenterBox {
	return func() *gtk.CenterBox {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f CenterBox) PaddingEnd(padding int) CenterBox {
	return func() *gtk.CenterBox {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f CenterBox) PaddingStart(padding int) CenterBox {
	return func() *gtk.CenterBox {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f CenterBox) PaddingTop(padding int) CenterBox {
	return func() *gtk.CenterBox {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f CenterBox) VPadding(padding int) CenterBox {
	return func() *gtk.CenterBox {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f CenterBox) BindVisible(state *state.State[bool]) CenterBox {
	return func() *gtk.CenterBox {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *gtk.CenterBox) {
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

func (f CenterBox) BindHMargin(state *state.State[int32]) CenterBox {
	return func() *gtk.CenterBox {
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

func (f CenterBox) BindMargin(state *state.State[int32]) CenterBox {
	return func() *gtk.CenterBox {
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

func (f CenterBox) BindMarginBottom(state *state.State[int32]) CenterBox {
	return func() *gtk.CenterBox {
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

func (f CenterBox) BindMarginEnd(state *state.State[int32]) CenterBox {
	return func() *gtk.CenterBox {
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

func (f CenterBox) BindMarginStart(state *state.State[int32]) CenterBox {
	return func() *gtk.CenterBox {
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

func (f CenterBox) BindMarginTop(state *state.State[int32]) CenterBox {
	return func() *gtk.CenterBox {
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

func (f CenterBox) BindSensitive(state *state.State[bool]) CenterBox {
	return func() *gtk.CenterBox {
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
