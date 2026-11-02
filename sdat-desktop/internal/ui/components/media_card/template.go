package media_card

import (
	"codeberg.org/dergs/tonearm/internal/resources"
	"codeberg.org/dergs/tonearm/internal/settings"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/dergs/tonearm/pkg/utils/imgutil"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"codeberg.org/puregotk/puregotk/v4/pango"
	"github.com/infinytum/injector"
)

func SubTitle(text string) schwifty.Label {
	return Label(text).
		FontWeight(400).
		MaxWidthChars(15).
		WithCSSClass("dimmed").
		HAlign(gtk.AlignStartValue).
		Ellipsis(pango.EllipsizeEndValue)
}

func Card[T any](title string, subTitle schwifty.Widgetable[T], cover tonearm.Cover) schwifty.Button {
	return Button().
		Child(
			VStack(
				Image().
					PixelSize(172).
					Background("var(--sidebar-bg-color)").
					FromPaintable(resources.MissingAlbum()).
					ConnectRealize(func(i gtk.Widget) {
						if settings.Performance().AllowMediaCardImages() {
							if cover.HasStatic() {
								injector.MustInject[*imgutil.ImgUtil]().LoadIntoImageCropped(cover.Static(), gtk.ImageNewFromInternalPtr(i.Ptr))
							} else {
								gtk.ImageNewFromInternalPtr(i.Ptr).SetFromPaintable(resources.MissingAlbum())
							}
						}
					}).CornerRadius(10).Overflow(gtk.OverflowHiddenValue),
				Label(title).
					WithCSSClass("heading").
					MarginTop(10).
					MaxWidthChars(15).
					HAlign(gtk.AlignStartValue).
					Ellipsis(pango.EllipsizeEndValue),
				subTitle.MarginTop(2),
			),
		).
		Padding(10).
		HExpand(false).
		VExpand(false).
		WithCSSClass("flat")
}
