package gtk

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type Image func() *gtk.Image

func (f Image) AddController(controller *gtk.EventController) Image {
	return func() *gtk.Image {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f Image) ConnectConstruct(cb func(*gtk.Image)) Image {
	return func() *gtk.Image {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f Image) ConnectDestroy(cb func(gtk.Widget)) Image {
	return func() *gtk.Image {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f Image) ConnectHide(cb func(gtk.Widget)) Image {
	return func() *gtk.Image {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f Image) ConnectMap(cb func(gtk.Widget)) Image {
	return func() *gtk.Image {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f Image) ConnectRealize(cb func(gtk.Widget)) Image {
	return func() *gtk.Image {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f Image) ConnectShow(cb func(gtk.Widget)) Image {
	return func() *gtk.Image {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f Image) ConnectUnmap(cb func(gtk.Widget)) Image {
	return func() *gtk.Image {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f Image) ConnectUnrealize(cb func(gtk.Widget)) Image {
	return func() *gtk.Image {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f Image) Controller(controller *gtk.EventController) Image {
	return func() *gtk.Image {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f Image) Focusable(focusable bool) Image {
	return func() *gtk.Image {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f Image) FocusOnClick(focusOnClick bool) Image {
	return func() *gtk.Image {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f Image) HAlign(align gtk.Align) Image {
	return func() *gtk.Image {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f Image) HExpand(expand bool) Image {
	return func() *gtk.Image {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f Image) HMargin(horizontal int32) Image {
	return func() *gtk.Image {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f Image) Margin(margin int32) Image {
	return func() *gtk.Image {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f Image) MarginBottom(bottom int32) Image {
	return func() *gtk.Image {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f Image) MarginEnd(end int32) Image {
	return func() *gtk.Image {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f Image) MarginStart(start int32) Image {
	return func() *gtk.Image {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f Image) MarginTop(top int32) Image {
	return func() *gtk.Image {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f Image) Opacity(opacity float64) Image {
	return func() *gtk.Image {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f Image) Overflow(overflow gtk.Overflow) Image {
	return func() *gtk.Image {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f Image) Sensitive(sensitive bool) Image {
	return func() *gtk.Image {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f Image) SizeRequest(width, height int32) Image {
	return func() *gtk.Image {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f Image) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f Image) VAlign(align gtk.Align) Image {
	return func() *gtk.Image {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f Image) VExpand(expand bool) Image {
	return func() *gtk.Image {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f Image) Visible(visible bool) Image {
	return func() *gtk.Image {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f Image) VMargin(vertical int32) Image {
	return func() *gtk.Image {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f Image) Background(color string) Image {
	return func() *gtk.Image {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f Image) CornerRadius(radius int) Image {
	return func() *gtk.Image {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f Image) CSS(css string) Image {
	return func() *gtk.Image {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f Image) BindCSSClass(state *state.State[string]) Image {
	return func() *gtk.Image {
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

func (f Image) WithCSSClass(className string) Image {
	return func() *gtk.Image {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f Image) CSSWithCallback(cb func(elementName string) string) Image {
	return func() *gtk.Image {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *gtk.Image) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f Image) HPadding(padding int) Image {
	return func() *gtk.Image {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f Image) MinHeight(minHeight int) Image {
	return func() *gtk.Image {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f Image) MinWidth(minWidth int) Image {
	return func() *gtk.Image {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f Image) Padding(padding int) Image {
	return func() *gtk.Image {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f Image) PaddingBottom(padding int) Image {
	return func() *gtk.Image {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f Image) PaddingEnd(padding int) Image {
	return func() *gtk.Image {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f Image) PaddingStart(padding int) Image {
	return func() *gtk.Image {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f Image) PaddingTop(padding int) Image {
	return func() *gtk.Image {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f Image) VPadding(padding int) Image {
	return func() *gtk.Image {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f Image) BindVisible(state *state.State[bool]) Image {
	return func() *gtk.Image {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *gtk.Image) {
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

func (f Image) BindHMargin(state *state.State[int32]) Image {
	return func() *gtk.Image {
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

func (f Image) BindMargin(state *state.State[int32]) Image {
	return func() *gtk.Image {
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

func (f Image) BindMarginBottom(state *state.State[int32]) Image {
	return func() *gtk.Image {
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

func (f Image) BindMarginEnd(state *state.State[int32]) Image {
	return func() *gtk.Image {
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

func (f Image) BindMarginStart(state *state.State[int32]) Image {
	return func() *gtk.Image {
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

func (f Image) BindMarginTop(state *state.State[int32]) Image {
	return func() *gtk.Image {
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

func (f Image) BindSensitive(state *state.State[bool]) Image {
	return func() *gtk.Image {
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
