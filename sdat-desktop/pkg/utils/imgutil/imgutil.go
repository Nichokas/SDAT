package imgutil

import (
	"time"

	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/gdk"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"github.com/jellydator/ttlcache/v3"
)

type ImgUtil struct {
	cache *ttlcache.Cache[string, *gdk.Texture]
}

func (i *ImgUtil) LoadIntoImage(url string, image *gtk.Image) {
	ref := weak.NewWidgetRef(&image.Widget)
	go func() {
		texture, err := i.Fetch(url)
		if err != nil {
			return
		}

		schwifty.OnMainThreadOnce(func(u uintptr) {
			ref.Use(func(widget *gtk.Widget) {
				gtk.ImageNewFromInternalPtr(widget.GoPointer()).SetFromPaintable(texture)
			})
			texture = nil
		}, 0)
	}()
}

func (i *ImgUtil) LoadIntoImageCropped(url string, image *gtk.Image) {
	ref := weak.NewWidgetRef(&image.Widget)
	go func() {
		cropped, err := i.FetchCropped(url)
		if err != nil {
			return
		}

		schwifty.OnMainThreadOnce(func(u uintptr) {
			ref.Use(func(widget *gtk.Widget) {
				gtk.ImageNewFromInternalPtr(widget.GoPointer()).SetFromPaintable(cropped)
			})
			cropped = nil
		}, 0)
	}()
}

func NewImgUtil() *ImgUtil {
	imgUtil := &ImgUtil{}
	imgUtil.cache = ttlcache.New[string, *gdk.Texture](
		ttlcache.WithTTL[string, *gdk.Texture](10*time.Minute),
		ttlcache.WithMaxCost(50*(1024.0*1024.0), func(item ttlcache.CostItem[string, *gdk.Texture]) uint64 {
			// Approximate the cost as the memory usage this GdkTexture is likely to occupy
			return uint64(item.Value.GetWidth()) * uint64(item.Value.GetHeight()) * 4
		}),
		ttlcache.WithLoader(imgUtil),
	)
	go imgUtil.cache.Start()
	return imgUtil
}
