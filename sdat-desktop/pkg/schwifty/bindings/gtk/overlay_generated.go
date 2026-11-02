package gtk

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type Overlay func() *gtk.Overlay

func (f Overlay) AddController(controller *gtk.EventController) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f Overlay) ConnectConstruct(cb func(*gtk.Overlay)) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f Overlay) ConnectDestroy(cb func(gtk.Widget)) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f Overlay) ConnectHide(cb func(gtk.Widget)) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f Overlay) ConnectMap(cb func(gtk.Widget)) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f Overlay) ConnectRealize(cb func(gtk.Widget)) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f Overlay) ConnectShow(cb func(gtk.Widget)) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f Overlay) ConnectUnmap(cb func(gtk.Widget)) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f Overlay) ConnectUnrealize(cb func(gtk.Widget)) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f Overlay) Controller(controller *gtk.EventController) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f Overlay) Focusable(focusable bool) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f Overlay) FocusOnClick(focusOnClick bool) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f Overlay) HAlign(align gtk.Align) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f Overlay) HExpand(expand bool) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f Overlay) HMargin(horizontal int32) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f Overlay) Margin(margin int32) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f Overlay) MarginBottom(bottom int32) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f Overlay) MarginEnd(end int32) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f Overlay) MarginStart(start int32) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f Overlay) MarginTop(top int32) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f Overlay) Opacity(opacity float64) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f Overlay) Overflow(overflow gtk.Overflow) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f Overlay) Sensitive(sensitive bool) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f Overlay) SizeRequest(width, height int32) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f Overlay) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f Overlay) VAlign(align gtk.Align) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f Overlay) VExpand(expand bool) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f Overlay) Visible(visible bool) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f Overlay) VMargin(vertical int32) Overlay {
	return func() *gtk.Overlay {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f Overlay) Background(color string) Overlay {
	return func() *gtk.Overlay {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f Overlay) CornerRadius(radius int) Overlay {
	return func() *gtk.Overlay {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f Overlay) CSS(css string) Overlay {
	return func() *gtk.Overlay {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f Overlay) BindCSSClass(state *state.State[string]) Overlay {
	return func() *gtk.Overlay {
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

func (f Overlay) WithCSSClass(className string) Overlay {
	return func() *gtk.Overlay {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f Overlay) CSSWithCallback(cb func(elementName string) string) Overlay {
	return func() *gtk.Overlay {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *gtk.Overlay) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f Overlay) HPadding(padding int) Overlay {
	return func() *gtk.Overlay {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f Overlay) MinHeight(minHeight int) Overlay {
	return func() *gtk.Overlay {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f Overlay) MinWidth(minWidth int) Overlay {
	return func() *gtk.Overlay {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f Overlay) Padding(padding int) Overlay {
	return func() *gtk.Overlay {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f Overlay) PaddingBottom(padding int) Overlay {
	return func() *gtk.Overlay {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f Overlay) PaddingEnd(padding int) Overlay {
	return func() *gtk.Overlay {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f Overlay) PaddingStart(padding int) Overlay {
	return func() *gtk.Overlay {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f Overlay) PaddingTop(padding int) Overlay {
	return func() *gtk.Overlay {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f Overlay) VPadding(padding int) Overlay {
	return func() *gtk.Overlay {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f Overlay) BindVisible(state *state.State[bool]) Overlay {
	return func() *gtk.Overlay {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *gtk.Overlay) {
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

func (f Overlay) BindHMargin(state *state.State[int32]) Overlay {
	return func() *gtk.Overlay {
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

func (f Overlay) BindMargin(state *state.State[int32]) Overlay {
	return func() *gtk.Overlay {
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

func (f Overlay) BindMarginBottom(state *state.State[int32]) Overlay {
	return func() *gtk.Overlay {
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

func (f Overlay) BindMarginEnd(state *state.State[int32]) Overlay {
	return func() *gtk.Overlay {
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

func (f Overlay) BindMarginStart(state *state.State[int32]) Overlay {
	return func() *gtk.Overlay {
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

func (f Overlay) BindMarginTop(state *state.State[int32]) Overlay {
	return func() *gtk.Overlay {
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

func (f Overlay) BindSensitive(state *state.State[bool]) Overlay {
	return func() *gtk.Overlay {
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
