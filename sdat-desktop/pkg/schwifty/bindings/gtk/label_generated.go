package gtk

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type Label func() *gtk.Label

func (f Label) AddController(controller *gtk.EventController) Label {
	return func() *gtk.Label {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f Label) ConnectConstruct(cb func(*gtk.Label)) Label {
	return func() *gtk.Label {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f Label) ConnectDestroy(cb func(gtk.Widget)) Label {
	return func() *gtk.Label {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f Label) ConnectHide(cb func(gtk.Widget)) Label {
	return func() *gtk.Label {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f Label) ConnectMap(cb func(gtk.Widget)) Label {
	return func() *gtk.Label {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f Label) ConnectRealize(cb func(gtk.Widget)) Label {
	return func() *gtk.Label {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f Label) ConnectShow(cb func(gtk.Widget)) Label {
	return func() *gtk.Label {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f Label) ConnectUnmap(cb func(gtk.Widget)) Label {
	return func() *gtk.Label {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f Label) ConnectUnrealize(cb func(gtk.Widget)) Label {
	return func() *gtk.Label {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f Label) Controller(controller *gtk.EventController) Label {
	return func() *gtk.Label {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f Label) Focusable(focusable bool) Label {
	return func() *gtk.Label {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f Label) FocusOnClick(focusOnClick bool) Label {
	return func() *gtk.Label {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f Label) HAlign(align gtk.Align) Label {
	return func() *gtk.Label {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f Label) HExpand(expand bool) Label {
	return func() *gtk.Label {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f Label) HMargin(horizontal int32) Label {
	return func() *gtk.Label {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f Label) Margin(margin int32) Label {
	return func() *gtk.Label {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f Label) MarginBottom(bottom int32) Label {
	return func() *gtk.Label {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f Label) MarginEnd(end int32) Label {
	return func() *gtk.Label {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f Label) MarginStart(start int32) Label {
	return func() *gtk.Label {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f Label) MarginTop(top int32) Label {
	return func() *gtk.Label {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f Label) Opacity(opacity float64) Label {
	return func() *gtk.Label {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f Label) Overflow(overflow gtk.Overflow) Label {
	return func() *gtk.Label {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f Label) Sensitive(sensitive bool) Label {
	return func() *gtk.Label {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f Label) SizeRequest(width, height int32) Label {
	return func() *gtk.Label {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f Label) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f Label) VAlign(align gtk.Align) Label {
	return func() *gtk.Label {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f Label) VExpand(expand bool) Label {
	return func() *gtk.Label {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f Label) Visible(visible bool) Label {
	return func() *gtk.Label {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f Label) VMargin(vertical int32) Label {
	return func() *gtk.Label {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f Label) Background(color string) Label {
	return func() *gtk.Label {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f Label) CornerRadius(radius int) Label {
	return func() *gtk.Label {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f Label) CSS(css string) Label {
	return func() *gtk.Label {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f Label) BindCSSClass(state *state.State[string]) Label {
	return func() *gtk.Label {
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

func (f Label) WithCSSClass(className string) Label {
	return func() *gtk.Label {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f Label) CSSWithCallback(cb func(elementName string) string) Label {
	return func() *gtk.Label {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *gtk.Label) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f Label) HPadding(padding int) Label {
	return func() *gtk.Label {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f Label) MinHeight(minHeight int) Label {
	return func() *gtk.Label {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f Label) MinWidth(minWidth int) Label {
	return func() *gtk.Label {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f Label) Padding(padding int) Label {
	return func() *gtk.Label {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f Label) PaddingBottom(padding int) Label {
	return func() *gtk.Label {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f Label) PaddingEnd(padding int) Label {
	return func() *gtk.Label {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f Label) PaddingStart(padding int) Label {
	return func() *gtk.Label {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f Label) PaddingTop(padding int) Label {
	return func() *gtk.Label {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f Label) VPadding(padding int) Label {
	return func() *gtk.Label {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f Label) BindVisible(state *state.State[bool]) Label {
	return func() *gtk.Label {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *gtk.Label) {
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

func (f Label) BindHMargin(state *state.State[int32]) Label {
	return func() *gtk.Label {
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

func (f Label) BindMargin(state *state.State[int32]) Label {
	return func() *gtk.Label {
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

func (f Label) BindMarginBottom(state *state.State[int32]) Label {
	return func() *gtk.Label {
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

func (f Label) BindMarginEnd(state *state.State[int32]) Label {
	return func() *gtk.Label {
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

func (f Label) BindMarginStart(state *state.State[int32]) Label {
	return func() *gtk.Label {
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

func (f Label) BindMarginTop(state *state.State[int32]) Label {
	return func() *gtk.Label {
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

func (f Label) BindSensitive(state *state.State[bool]) Label {
	return func() *gtk.Label {
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
