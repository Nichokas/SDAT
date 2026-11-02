package adw

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type PreferencesPage func() *adw.PreferencesPage

func (f PreferencesPage) AddController(controller *gtk.EventController) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f PreferencesPage) ConnectConstruct(cb func(*adw.PreferencesPage)) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f PreferencesPage) ConnectDestroy(cb func(gtk.Widget)) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f PreferencesPage) ConnectHide(cb func(gtk.Widget)) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f PreferencesPage) ConnectMap(cb func(gtk.Widget)) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f PreferencesPage) ConnectRealize(cb func(gtk.Widget)) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f PreferencesPage) ConnectShow(cb func(gtk.Widget)) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f PreferencesPage) ConnectUnmap(cb func(gtk.Widget)) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f PreferencesPage) ConnectUnrealize(cb func(gtk.Widget)) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f PreferencesPage) Controller(controller *gtk.EventController) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f PreferencesPage) Focusable(focusable bool) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f PreferencesPage) FocusOnClick(focusOnClick bool) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f PreferencesPage) HAlign(align gtk.Align) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f PreferencesPage) HExpand(expand bool) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f PreferencesPage) HMargin(horizontal int32) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f PreferencesPage) Margin(margin int32) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f PreferencesPage) MarginBottom(bottom int32) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f PreferencesPage) MarginEnd(end int32) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f PreferencesPage) MarginStart(start int32) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f PreferencesPage) MarginTop(top int32) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f PreferencesPage) Opacity(opacity float64) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f PreferencesPage) Overflow(overflow gtk.Overflow) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f PreferencesPage) Sensitive(sensitive bool) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f PreferencesPage) SizeRequest(width, height int32) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f PreferencesPage) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f PreferencesPage) VAlign(align gtk.Align) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f PreferencesPage) VExpand(expand bool) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f PreferencesPage) Visible(visible bool) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f PreferencesPage) VMargin(vertical int32) PreferencesPage {
	return func() *adw.PreferencesPage {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f PreferencesPage) Background(color string) PreferencesPage {
	return func() *adw.PreferencesPage {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f PreferencesPage) CornerRadius(radius int) PreferencesPage {
	return func() *adw.PreferencesPage {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f PreferencesPage) CSS(css string) PreferencesPage {
	return func() *adw.PreferencesPage {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f PreferencesPage) BindCSSClass(state *state.State[string]) PreferencesPage {
	return func() *adw.PreferencesPage {
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

func (f PreferencesPage) WithCSSClass(className string) PreferencesPage {
	return func() *adw.PreferencesPage {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f PreferencesPage) CSSWithCallback(cb func(elementName string) string) PreferencesPage {
	return func() *adw.PreferencesPage {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *adw.PreferencesPage) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f PreferencesPage) HPadding(padding int) PreferencesPage {
	return func() *adw.PreferencesPage {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f PreferencesPage) MinHeight(minHeight int) PreferencesPage {
	return func() *adw.PreferencesPage {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f PreferencesPage) MinWidth(minWidth int) PreferencesPage {
	return func() *adw.PreferencesPage {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f PreferencesPage) Padding(padding int) PreferencesPage {
	return func() *adw.PreferencesPage {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f PreferencesPage) PaddingBottom(padding int) PreferencesPage {
	return func() *adw.PreferencesPage {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f PreferencesPage) PaddingEnd(padding int) PreferencesPage {
	return func() *adw.PreferencesPage {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f PreferencesPage) PaddingStart(padding int) PreferencesPage {
	return func() *adw.PreferencesPage {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f PreferencesPage) PaddingTop(padding int) PreferencesPage {
	return func() *adw.PreferencesPage {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f PreferencesPage) VPadding(padding int) PreferencesPage {
	return func() *adw.PreferencesPage {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f PreferencesPage) BindVisible(state *state.State[bool]) PreferencesPage {
	return func() *adw.PreferencesPage {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *adw.PreferencesPage) {
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

func (f PreferencesPage) BindHMargin(state *state.State[int32]) PreferencesPage {
	return func() *adw.PreferencesPage {
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

func (f PreferencesPage) BindMargin(state *state.State[int32]) PreferencesPage {
	return func() *adw.PreferencesPage {
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

func (f PreferencesPage) BindMarginBottom(state *state.State[int32]) PreferencesPage {
	return func() *adw.PreferencesPage {
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

func (f PreferencesPage) BindMarginEnd(state *state.State[int32]) PreferencesPage {
	return func() *adw.PreferencesPage {
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

func (f PreferencesPage) BindMarginStart(state *state.State[int32]) PreferencesPage {
	return func() *adw.PreferencesPage {
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

func (f PreferencesPage) BindMarginTop(state *state.State[int32]) PreferencesPage {
	return func() *adw.PreferencesPage {
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

func (f PreferencesPage) BindSensitive(state *state.State[bool]) PreferencesPage {
	return func() *adw.PreferencesPage {
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
