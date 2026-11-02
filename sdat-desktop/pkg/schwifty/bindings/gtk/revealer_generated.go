package gtk

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type Revealer func() *gtk.Revealer

func (f Revealer) AddController(controller *gtk.EventController) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f Revealer) ConnectConstruct(cb func(*gtk.Revealer)) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f Revealer) ConnectDestroy(cb func(gtk.Widget)) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f Revealer) ConnectHide(cb func(gtk.Widget)) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f Revealer) ConnectMap(cb func(gtk.Widget)) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f Revealer) ConnectRealize(cb func(gtk.Widget)) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f Revealer) ConnectShow(cb func(gtk.Widget)) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f Revealer) ConnectUnmap(cb func(gtk.Widget)) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f Revealer) ConnectUnrealize(cb func(gtk.Widget)) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f Revealer) Controller(controller *gtk.EventController) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f Revealer) Focusable(focusable bool) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f Revealer) FocusOnClick(focusOnClick bool) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f Revealer) HAlign(align gtk.Align) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f Revealer) HExpand(expand bool) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f Revealer) HMargin(horizontal int32) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f Revealer) Margin(margin int32) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f Revealer) MarginBottom(bottom int32) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f Revealer) MarginEnd(end int32) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f Revealer) MarginStart(start int32) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f Revealer) MarginTop(top int32) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f Revealer) Opacity(opacity float64) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f Revealer) Overflow(overflow gtk.Overflow) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f Revealer) Sensitive(sensitive bool) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f Revealer) SizeRequest(width, height int32) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f Revealer) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f Revealer) VAlign(align gtk.Align) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f Revealer) VExpand(expand bool) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f Revealer) Visible(visible bool) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f Revealer) VMargin(vertical int32) Revealer {
	return func() *gtk.Revealer {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f Revealer) Background(color string) Revealer {
	return func() *gtk.Revealer {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f Revealer) CornerRadius(radius int) Revealer {
	return func() *gtk.Revealer {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f Revealer) CSS(css string) Revealer {
	return func() *gtk.Revealer {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f Revealer) BindCSSClass(state *state.State[string]) Revealer {
	return func() *gtk.Revealer {
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

func (f Revealer) WithCSSClass(className string) Revealer {
	return func() *gtk.Revealer {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f Revealer) CSSWithCallback(cb func(elementName string) string) Revealer {
	return func() *gtk.Revealer {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *gtk.Revealer) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f Revealer) HPadding(padding int) Revealer {
	return func() *gtk.Revealer {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f Revealer) MinHeight(minHeight int) Revealer {
	return func() *gtk.Revealer {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f Revealer) MinWidth(minWidth int) Revealer {
	return func() *gtk.Revealer {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f Revealer) Padding(padding int) Revealer {
	return func() *gtk.Revealer {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f Revealer) PaddingBottom(padding int) Revealer {
	return func() *gtk.Revealer {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f Revealer) PaddingEnd(padding int) Revealer {
	return func() *gtk.Revealer {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f Revealer) PaddingStart(padding int) Revealer {
	return func() *gtk.Revealer {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f Revealer) PaddingTop(padding int) Revealer {
	return func() *gtk.Revealer {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f Revealer) VPadding(padding int) Revealer {
	return func() *gtk.Revealer {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f Revealer) BindVisible(state *state.State[bool]) Revealer {
	return func() *gtk.Revealer {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *gtk.Revealer) {
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

func (f Revealer) BindHMargin(state *state.State[int32]) Revealer {
	return func() *gtk.Revealer {
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

func (f Revealer) BindMargin(state *state.State[int32]) Revealer {
	return func() *gtk.Revealer {
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

func (f Revealer) BindMarginBottom(state *state.State[int32]) Revealer {
	return func() *gtk.Revealer {
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

func (f Revealer) BindMarginEnd(state *state.State[int32]) Revealer {
	return func() *gtk.Revealer {
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

func (f Revealer) BindMarginStart(state *state.State[int32]) Revealer {
	return func() *gtk.Revealer {
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

func (f Revealer) BindMarginTop(state *state.State[int32]) Revealer {
	return func() *gtk.Revealer {
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

func (f Revealer) BindSensitive(state *state.State[bool]) Revealer {
	return func() *gtk.Revealer {
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
