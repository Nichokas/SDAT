package gtk

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type ProgressBar func() *gtk.ProgressBar

func (f ProgressBar) AddController(controller *gtk.EventController) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f ProgressBar) ConnectConstruct(cb func(*gtk.ProgressBar)) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f ProgressBar) ConnectDestroy(cb func(gtk.Widget)) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f ProgressBar) ConnectHide(cb func(gtk.Widget)) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f ProgressBar) ConnectMap(cb func(gtk.Widget)) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f ProgressBar) ConnectRealize(cb func(gtk.Widget)) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f ProgressBar) ConnectShow(cb func(gtk.Widget)) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f ProgressBar) ConnectUnmap(cb func(gtk.Widget)) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f ProgressBar) ConnectUnrealize(cb func(gtk.Widget)) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f ProgressBar) Controller(controller *gtk.EventController) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f ProgressBar) Focusable(focusable bool) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f ProgressBar) FocusOnClick(focusOnClick bool) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f ProgressBar) HAlign(align gtk.Align) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f ProgressBar) HExpand(expand bool) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f ProgressBar) HMargin(horizontal int32) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f ProgressBar) Margin(margin int32) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f ProgressBar) MarginBottom(bottom int32) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f ProgressBar) MarginEnd(end int32) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f ProgressBar) MarginStart(start int32) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f ProgressBar) MarginTop(top int32) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f ProgressBar) Opacity(opacity float64) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f ProgressBar) Overflow(overflow gtk.Overflow) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f ProgressBar) Sensitive(sensitive bool) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f ProgressBar) SizeRequest(width, height int32) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f ProgressBar) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f ProgressBar) VAlign(align gtk.Align) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f ProgressBar) VExpand(expand bool) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f ProgressBar) Visible(visible bool) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f ProgressBar) VMargin(vertical int32) ProgressBar {
	return func() *gtk.ProgressBar {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f ProgressBar) Background(color string) ProgressBar {
	return func() *gtk.ProgressBar {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f ProgressBar) CornerRadius(radius int) ProgressBar {
	return func() *gtk.ProgressBar {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f ProgressBar) CSS(css string) ProgressBar {
	return func() *gtk.ProgressBar {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f ProgressBar) BindCSSClass(state *state.State[string]) ProgressBar {
	return func() *gtk.ProgressBar {
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

func (f ProgressBar) WithCSSClass(className string) ProgressBar {
	return func() *gtk.ProgressBar {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f ProgressBar) CSSWithCallback(cb func(elementName string) string) ProgressBar {
	return func() *gtk.ProgressBar {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *gtk.ProgressBar) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f ProgressBar) HPadding(padding int) ProgressBar {
	return func() *gtk.ProgressBar {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f ProgressBar) MinHeight(minHeight int) ProgressBar {
	return func() *gtk.ProgressBar {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f ProgressBar) MinWidth(minWidth int) ProgressBar {
	return func() *gtk.ProgressBar {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f ProgressBar) Padding(padding int) ProgressBar {
	return func() *gtk.ProgressBar {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f ProgressBar) PaddingBottom(padding int) ProgressBar {
	return func() *gtk.ProgressBar {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f ProgressBar) PaddingEnd(padding int) ProgressBar {
	return func() *gtk.ProgressBar {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f ProgressBar) PaddingStart(padding int) ProgressBar {
	return func() *gtk.ProgressBar {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f ProgressBar) PaddingTop(padding int) ProgressBar {
	return func() *gtk.ProgressBar {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f ProgressBar) VPadding(padding int) ProgressBar {
	return func() *gtk.ProgressBar {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f ProgressBar) BindVisible(state *state.State[bool]) ProgressBar {
	return func() *gtk.ProgressBar {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *gtk.ProgressBar) {
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

func (f ProgressBar) BindHMargin(state *state.State[int32]) ProgressBar {
	return func() *gtk.ProgressBar {
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

func (f ProgressBar) BindMargin(state *state.State[int32]) ProgressBar {
	return func() *gtk.ProgressBar {
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

func (f ProgressBar) BindMarginBottom(state *state.State[int32]) ProgressBar {
	return func() *gtk.ProgressBar {
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

func (f ProgressBar) BindMarginEnd(state *state.State[int32]) ProgressBar {
	return func() *gtk.ProgressBar {
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

func (f ProgressBar) BindMarginStart(state *state.State[int32]) ProgressBar {
	return func() *gtk.ProgressBar {
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

func (f ProgressBar) BindMarginTop(state *state.State[int32]) ProgressBar {
	return func() *gtk.ProgressBar {
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

func (f ProgressBar) BindSensitive(state *state.State[bool]) ProgressBar {
	return func() *gtk.ProgressBar {
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
