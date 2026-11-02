package gtk

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type Widget func() *WrappedWidget

func (f Widget) AddController(controller *gtk.EventController) Widget {
	return func() *WrappedWidget {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f Widget) ConnectConstruct(cb func(*WrappedWidget)) Widget {
	return func() *WrappedWidget {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f Widget) ConnectDestroy(cb func(gtk.Widget)) Widget {
	return func() *WrappedWidget {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f Widget) ConnectHide(cb func(gtk.Widget)) Widget {
	return func() *WrappedWidget {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f Widget) ConnectMap(cb func(gtk.Widget)) Widget {
	return func() *WrappedWidget {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f Widget) ConnectRealize(cb func(gtk.Widget)) Widget {
	return func() *WrappedWidget {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f Widget) ConnectShow(cb func(gtk.Widget)) Widget {
	return func() *WrappedWidget {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f Widget) ConnectUnmap(cb func(gtk.Widget)) Widget {
	return func() *WrappedWidget {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f Widget) ConnectUnrealize(cb func(gtk.Widget)) Widget {
	return func() *WrappedWidget {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f Widget) Controller(controller *gtk.EventController) Widget {
	return func() *WrappedWidget {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f Widget) Focusable(focusable bool) Widget {
	return func() *WrappedWidget {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f Widget) FocusOnClick(focusOnClick bool) Widget {
	return func() *WrappedWidget {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f Widget) HAlign(align gtk.Align) Widget {
	return func() *WrappedWidget {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f Widget) HExpand(expand bool) Widget {
	return func() *WrappedWidget {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f Widget) HMargin(horizontal int32) Widget {
	return func() *WrappedWidget {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f Widget) Margin(margin int32) Widget {
	return func() *WrappedWidget {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f Widget) MarginBottom(bottom int32) Widget {
	return func() *WrappedWidget {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f Widget) MarginEnd(end int32) Widget {
	return func() *WrappedWidget {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f Widget) MarginStart(start int32) Widget {
	return func() *WrappedWidget {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f Widget) MarginTop(top int32) Widget {
	return func() *WrappedWidget {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f Widget) Opacity(opacity float64) Widget {
	return func() *WrappedWidget {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f Widget) Overflow(overflow gtk.Overflow) Widget {
	return func() *WrappedWidget {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f Widget) Sensitive(sensitive bool) Widget {
	return func() *WrappedWidget {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f Widget) SizeRequest(width, height int32) Widget {
	return func() *WrappedWidget {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f Widget) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f Widget) VAlign(align gtk.Align) Widget {
	return func() *WrappedWidget {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f Widget) VExpand(expand bool) Widget {
	return func() *WrappedWidget {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f Widget) Visible(visible bool) Widget {
	return func() *WrappedWidget {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f Widget) VMargin(vertical int32) Widget {
	return func() *WrappedWidget {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f Widget) Background(color string) Widget {
	return func() *WrappedWidget {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f Widget) CornerRadius(radius int) Widget {
	return func() *WrappedWidget {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f Widget) CSS(css string) Widget {
	return func() *WrappedWidget {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f Widget) BindCSSClass(state *state.State[string]) Widget {
	return func() *WrappedWidget {
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

func (f Widget) WithCSSClass(className string) Widget {
	return func() *WrappedWidget {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f Widget) CSSWithCallback(cb func(elementName string) string) Widget {
	return func() *WrappedWidget {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *WrappedWidget) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f Widget) HPadding(padding int) Widget {
	return func() *WrappedWidget {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f Widget) MinHeight(minHeight int) Widget {
	return func() *WrappedWidget {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f Widget) MinWidth(minWidth int) Widget {
	return func() *WrappedWidget {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f Widget) Padding(padding int) Widget {
	return func() *WrappedWidget {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f Widget) PaddingBottom(padding int) Widget {
	return func() *WrappedWidget {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f Widget) PaddingEnd(padding int) Widget {
	return func() *WrappedWidget {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f Widget) PaddingStart(padding int) Widget {
	return func() *WrappedWidget {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f Widget) PaddingTop(padding int) Widget {
	return func() *WrappedWidget {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f Widget) VPadding(padding int) Widget {
	return func() *WrappedWidget {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f Widget) BindVisible(state *state.State[bool]) Widget {
	return func() *WrappedWidget {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *WrappedWidget) {
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

func (f Widget) BindHMargin(state *state.State[int32]) Widget {
	return func() *WrappedWidget {
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

func (f Widget) BindMargin(state *state.State[int32]) Widget {
	return func() *WrappedWidget {
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

func (f Widget) BindMarginBottom(state *state.State[int32]) Widget {
	return func() *WrappedWidget {
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

func (f Widget) BindMarginEnd(state *state.State[int32]) Widget {
	return func() *WrappedWidget {
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

func (f Widget) BindMarginStart(state *state.State[int32]) Widget {
	return func() *WrappedWidget {
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

func (f Widget) BindMarginTop(state *state.State[int32]) Widget {
	return func() *WrappedWidget {
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

func (f Widget) BindSensitive(state *state.State[bool]) Widget {
	return func() *WrappedWidget {
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
