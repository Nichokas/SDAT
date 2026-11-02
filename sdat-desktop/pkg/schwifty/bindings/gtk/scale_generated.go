package gtk

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type Scale func() *gtk.Scale

func (f Scale) AddController(controller *gtk.EventController) Scale {
	return func() *gtk.Scale {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f Scale) ConnectConstruct(cb func(*gtk.Scale)) Scale {
	return func() *gtk.Scale {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f Scale) ConnectDestroy(cb func(gtk.Widget)) Scale {
	return func() *gtk.Scale {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f Scale) ConnectHide(cb func(gtk.Widget)) Scale {
	return func() *gtk.Scale {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f Scale) ConnectMap(cb func(gtk.Widget)) Scale {
	return func() *gtk.Scale {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f Scale) ConnectRealize(cb func(gtk.Widget)) Scale {
	return func() *gtk.Scale {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f Scale) ConnectShow(cb func(gtk.Widget)) Scale {
	return func() *gtk.Scale {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f Scale) ConnectUnmap(cb func(gtk.Widget)) Scale {
	return func() *gtk.Scale {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f Scale) ConnectUnrealize(cb func(gtk.Widget)) Scale {
	return func() *gtk.Scale {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f Scale) Controller(controller *gtk.EventController) Scale {
	return func() *gtk.Scale {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f Scale) Focusable(focusable bool) Scale {
	return func() *gtk.Scale {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f Scale) FocusOnClick(focusOnClick bool) Scale {
	return func() *gtk.Scale {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f Scale) HAlign(align gtk.Align) Scale {
	return func() *gtk.Scale {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f Scale) HExpand(expand bool) Scale {
	return func() *gtk.Scale {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f Scale) HMargin(horizontal int32) Scale {
	return func() *gtk.Scale {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f Scale) Margin(margin int32) Scale {
	return func() *gtk.Scale {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f Scale) MarginBottom(bottom int32) Scale {
	return func() *gtk.Scale {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f Scale) MarginEnd(end int32) Scale {
	return func() *gtk.Scale {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f Scale) MarginStart(start int32) Scale {
	return func() *gtk.Scale {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f Scale) MarginTop(top int32) Scale {
	return func() *gtk.Scale {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f Scale) Opacity(opacity float64) Scale {
	return func() *gtk.Scale {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f Scale) Overflow(overflow gtk.Overflow) Scale {
	return func() *gtk.Scale {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f Scale) Sensitive(sensitive bool) Scale {
	return func() *gtk.Scale {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f Scale) SizeRequest(width, height int32) Scale {
	return func() *gtk.Scale {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f Scale) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f Scale) VAlign(align gtk.Align) Scale {
	return func() *gtk.Scale {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f Scale) VExpand(expand bool) Scale {
	return func() *gtk.Scale {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f Scale) Visible(visible bool) Scale {
	return func() *gtk.Scale {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f Scale) VMargin(vertical int32) Scale {
	return func() *gtk.Scale {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f Scale) Background(color string) Scale {
	return func() *gtk.Scale {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f Scale) CornerRadius(radius int) Scale {
	return func() *gtk.Scale {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f Scale) CSS(css string) Scale {
	return func() *gtk.Scale {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f Scale) BindCSSClass(state *state.State[string]) Scale {
	return func() *gtk.Scale {
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

func (f Scale) WithCSSClass(className string) Scale {
	return func() *gtk.Scale {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f Scale) CSSWithCallback(cb func(elementName string) string) Scale {
	return func() *gtk.Scale {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *gtk.Scale) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f Scale) HPadding(padding int) Scale {
	return func() *gtk.Scale {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f Scale) MinHeight(minHeight int) Scale {
	return func() *gtk.Scale {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f Scale) MinWidth(minWidth int) Scale {
	return func() *gtk.Scale {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f Scale) Padding(padding int) Scale {
	return func() *gtk.Scale {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f Scale) PaddingBottom(padding int) Scale {
	return func() *gtk.Scale {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f Scale) PaddingEnd(padding int) Scale {
	return func() *gtk.Scale {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f Scale) PaddingStart(padding int) Scale {
	return func() *gtk.Scale {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f Scale) PaddingTop(padding int) Scale {
	return func() *gtk.Scale {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f Scale) VPadding(padding int) Scale {
	return func() *gtk.Scale {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f Scale) BindVisible(state *state.State[bool]) Scale {
	return func() *gtk.Scale {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *gtk.Scale) {
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

func (f Scale) BindHMargin(state *state.State[int32]) Scale {
	return func() *gtk.Scale {
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

func (f Scale) BindMargin(state *state.State[int32]) Scale {
	return func() *gtk.Scale {
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

func (f Scale) BindMarginBottom(state *state.State[int32]) Scale {
	return func() *gtk.Scale {
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

func (f Scale) BindMarginEnd(state *state.State[int32]) Scale {
	return func() *gtk.Scale {
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

func (f Scale) BindMarginStart(state *state.State[int32]) Scale {
	return func() *gtk.Scale {
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

func (f Scale) BindMarginTop(state *state.State[int32]) Scale {
	return func() *gtk.Scale {
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

func (f Scale) BindSensitive(state *state.State[bool]) Scale {
	return func() *gtk.Scale {
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
