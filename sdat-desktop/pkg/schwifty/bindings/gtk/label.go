package gtk

import (
	"fmt"

	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"codeberg.org/puregotk/puregotk/v4/pango"
)

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen Label *gtk.Label gtk

func (f Label) Color(color string) Label {
	return func() *gtk.Label {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { color: %s; }", elementName, color)
		})()
	}
}

// Sets the mode used to ellipsize the text.
//
// The text will be ellipsized if there is not
// enough space to render the entire string.
func (f Label) Ellipsis(ellipsis pango.EllipsizeMode) Label {
	return func() *gtk.Label {
		label := f()
		label.SetEllipsize(ellipsis)
		return label
	}
}

// Sets the font size for the label text.
//
// The size parameter determines the size of the text in points.
// Common values include 12 for small text and 16 for larger text,
// though the exact range and supported values depend on the font family.
//
// This function modifies the font attributes of the label to apply
// the specified size to the displayed text.
func (f Label) FontSize(size int) Label {
	return func() *gtk.Label {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { font-size: %dpx; }", elementName, size)
		})()
	}
}

// Sets the font weight for the label text.
//
// The weight parameter determines how bold or light the text appears.
// Common values include 400 for normal weight and 700 for bold weight,
// though the exact range and supported values depend on the font family.
//
// This function modifies the font attributes of the label to apply
// the specified weight to the displayed text.
func (f Label) FontWeight(weight int) Label {
	return func() *gtk.Label {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { font-weight: %d; }", elementName, weight)
		})()
	}
}

// Sets the alignment of lines in the label relative to each other.
//
// This function has no effect on labels containing only a single line.
//
// [enum@Gtk.Justification.left] is the default value when the widget
// is first created with [ctor@Gtk.Label.new].
//
// If you instead want to set the alignment of the label as a whole,
// use [method@Gtk.Widget.set_halign] instead.
func (f Label) Justify(justify gtk.Justification) Label {
	return func() *gtk.Label {
		label := f()
		label.SetJustify(justify)
		return label
	}
}

func (f Label) LineHeight(height float64) Label {
	return func() *gtk.Label {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { line-height: %.2f; }", elementName, height)
		})()
	}
}

// Sets the text for the label.
//
// It overwrites any text that was there before and clears any
// previously set mnemonic accelerators, and sets the
// [property@Gtk.Label:use-underline] and
// [property@Gtk.Label:use-markup] properties to false.
//
// Also see [method@Gtk.Label.set_markup].
func (f Label) Text(text string) Label {
	return func() *gtk.Label {
		label := f()
		label.SetText(text)
		return label
	}
}

func (f Label) BindText(state *state.State[string]) Label {
	return func() *gtk.Label {
		var callbackId string
		var ref weak.WidgetRef
		return f.ConnectRealize(func(w gtk.Widget) {
			ref = weak.NewWidgetRef(&w)
			callbackId = state.AddCallback(func(newValue string) {
				callback.OnMainThreadOncePure(func() {
					if obj := ref.Get(); obj != nil {
						defer obj.Unref()
						gtk.LabelNewFromInternalPtr(obj.Ptr).SetText(newValue)
					}
				})
			})
		}).ConnectUnrealize(func(w gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}

func (f Label) Lines(lines int32) Label {
	return func() *gtk.Label {
		widget := f()
		widget.SetLines(lines)
		return widget
	}
}

func (f Label) MaxWidthChars(chars int32) Label {
	return func() *gtk.Label {
		widget := f()
		widget.SetMaxWidthChars(chars)
		return widget
	}
}

func (f Label) TextDecoration(decoration string) Label {
	return func() *gtk.Label {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { text-decoration: %s; }", elementName, decoration)
		})()
	}
}

// Toggles line wrapping within the label.
//
// True makes it break lines if text exceeds the widget’s size.
// false lets the text get cut off by the edge of the widget if
// it exceeds the widget size.
//
// Note that setting line wrapping to true does not make the label
// wrap at its parent widget’s width, because GTK widgets conceptually
// can’t make their requisition depend on the parent  widget’s size.
// For a label that wraps at a specific position, set the label’s width
// using [method@Gtk.Widget.set_size_request].
func (f Label) Wrap(wrap bool) Label {
	return func() *gtk.Label {
		label := f()
		label.SetWrap(wrap)
		return label
	}
}

// Controls how line wrapping is done.
//
// This only affects the label if line wrapping is on. (See
// [method@Gtk.Label.set_wrap])
//
// The default is [enum@Pango.WrapMode.word], which means
// wrap on word boundaries.
//
// For sizing behavior, also consider the
// [property@Gtk.Label:natural-wrap-mode] property.
func (f Label) WrapMode(wrapMode pango.WrapMode) Label {
	return func() *gtk.Label {
		label := f()
		label.SetWrapMode(wrapMode)
		return label
	}
}
