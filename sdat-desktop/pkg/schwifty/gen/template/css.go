package schwifty

import (
	"fmt"

	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

func (f TEMPLATE_TYPE) Background(color string) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f TEMPLATE_TYPE) CornerRadius(radius int) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f TEMPLATE_TYPE) CSS(css string) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f TEMPLATE_TYPE) BindCSSClass(state *state.State[string]) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
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

func (f TEMPLATE_TYPE) WithCSSClass(className string) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f TEMPLATE_TYPE) CSSWithCallback(cb func(elementName string) string) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t TEMPLATE_BASE_TYPE) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f TEMPLATE_TYPE) HPadding(padding int) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f TEMPLATE_TYPE) MinHeight(minHeight int) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f TEMPLATE_TYPE) MinWidth(minWidth int) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f TEMPLATE_TYPE) Padding(padding int) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f TEMPLATE_TYPE) PaddingBottom(padding int) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f TEMPLATE_TYPE) PaddingEnd(padding int) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f TEMPLATE_TYPE) PaddingStart(padding int) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f TEMPLATE_TYPE) PaddingTop(padding int) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f TEMPLATE_TYPE) VPadding(padding int) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}
