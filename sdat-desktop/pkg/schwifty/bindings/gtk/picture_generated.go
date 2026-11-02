package gtk

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type Picture func() *gtk.Picture

func (f Picture) AddController(controller *gtk.EventController) Picture {
	return func() *gtk.Picture {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f Picture) ConnectConstruct(cb func(*gtk.Picture)) Picture {
	return func() *gtk.Picture {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f Picture) ConnectDestroy(cb func(gtk.Widget)) Picture {
	return func() *gtk.Picture {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f Picture) ConnectHide(cb func(gtk.Widget)) Picture {
	return func() *gtk.Picture {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f Picture) ConnectMap(cb func(gtk.Widget)) Picture {
	return func() *gtk.Picture {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f Picture) ConnectRealize(cb func(gtk.Widget)) Picture {
	return func() *gtk.Picture {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f Picture) ConnectShow(cb func(gtk.Widget)) Picture {
	return func() *gtk.Picture {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f Picture) ConnectUnmap(cb func(gtk.Widget)) Picture {
	return func() *gtk.Picture {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f Picture) ConnectUnrealize(cb func(gtk.Widget)) Picture {
	return func() *gtk.Picture {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f Picture) Controller(controller *gtk.EventController) Picture {
	return func() *gtk.Picture {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f Picture) Focusable(focusable bool) Picture {
	return func() *gtk.Picture {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f Picture) FocusOnClick(focusOnClick bool) Picture {
	return func() *gtk.Picture {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f Picture) HAlign(align gtk.Align) Picture {
	return func() *gtk.Picture {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f Picture) HExpand(expand bool) Picture {
	return func() *gtk.Picture {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f Picture) HMargin(horizontal int32) Picture {
	return func() *gtk.Picture {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f Picture) Margin(margin int32) Picture {
	return func() *gtk.Picture {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f Picture) MarginBottom(bottom int32) Picture {
	return func() *gtk.Picture {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f Picture) MarginEnd(end int32) Picture {
	return func() *gtk.Picture {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f Picture) MarginStart(start int32) Picture {
	return func() *gtk.Picture {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f Picture) MarginTop(top int32) Picture {
	return func() *gtk.Picture {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f Picture) Opacity(opacity float64) Picture {
	return func() *gtk.Picture {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f Picture) Overflow(overflow gtk.Overflow) Picture {
	return func() *gtk.Picture {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f Picture) Sensitive(sensitive bool) Picture {
	return func() *gtk.Picture {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f Picture) SizeRequest(width, height int32) Picture {
	return func() *gtk.Picture {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f Picture) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f Picture) VAlign(align gtk.Align) Picture {
	return func() *gtk.Picture {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f Picture) VExpand(expand bool) Picture {
	return func() *gtk.Picture {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f Picture) Visible(visible bool) Picture {
	return func() *gtk.Picture {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f Picture) VMargin(vertical int32) Picture {
	return func() *gtk.Picture {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f Picture) Background(color string) Picture {
	return func() *gtk.Picture {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f Picture) CornerRadius(radius int) Picture {
	return func() *gtk.Picture {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f Picture) CSS(css string) Picture {
	return func() *gtk.Picture {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f Picture) BindCSSClass(state *state.State[string]) Picture {
	return func() *gtk.Picture {
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

func (f Picture) WithCSSClass(className string) Picture {
	return func() *gtk.Picture {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f Picture) CSSWithCallback(cb func(elementName string) string) Picture {
	return func() *gtk.Picture {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *gtk.Picture) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f Picture) HPadding(padding int) Picture {
	return func() *gtk.Picture {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f Picture) MinHeight(minHeight int) Picture {
	return func() *gtk.Picture {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f Picture) MinWidth(minWidth int) Picture {
	return func() *gtk.Picture {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f Picture) Padding(padding int) Picture {
	return func() *gtk.Picture {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f Picture) PaddingBottom(padding int) Picture {
	return func() *gtk.Picture {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f Picture) PaddingEnd(padding int) Picture {
	return func() *gtk.Picture {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f Picture) PaddingStart(padding int) Picture {
	return func() *gtk.Picture {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f Picture) PaddingTop(padding int) Picture {
	return func() *gtk.Picture {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f Picture) VPadding(padding int) Picture {
	return func() *gtk.Picture {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f Picture) BindVisible(state *state.State[bool]) Picture {
	return func() *gtk.Picture {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *gtk.Picture) {
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

func (f Picture) BindHMargin(state *state.State[int32]) Picture {
	return func() *gtk.Picture {
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

func (f Picture) BindMargin(state *state.State[int32]) Picture {
	return func() *gtk.Picture {
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

func (f Picture) BindMarginBottom(state *state.State[int32]) Picture {
	return func() *gtk.Picture {
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

func (f Picture) BindMarginEnd(state *state.State[int32]) Picture {
	return func() *gtk.Picture {
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

func (f Picture) BindMarginStart(state *state.State[int32]) Picture {
	return func() *gtk.Picture {
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

func (f Picture) BindMarginTop(state *state.State[int32]) Picture {
	return func() *gtk.Picture {
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

func (f Picture) BindSensitive(state *state.State[bool]) Picture {
	return func() *gtk.Picture {
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
