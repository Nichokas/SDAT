package imgutil

import (
	"codeberg.org/dergs/tonearm/internal/settings"
	"codeberg.org/dergs/tonearm/pkg/schwifty/tracking"
	"codeberg.org/puregotk/puregotk/v4/gdk"
	"codeberg.org/puregotk/puregotk/v4/glib"
	"github.com/jellydator/ttlcache/v3"
)

func (i *ImgUtil) Load(c *ttlcache.Cache[string, *gdk.Texture], key string) *ttlcache.Item[string, *gdk.Texture] {
	image, err := i.fetch(key)
	if err != nil {
		return nil
	}

	gBytes := glib.NewBytes(image, uint(len(image)))
	texture, err := gdk.NewTextureFromBytes(gBytes)
	gBytes.Unref()

	if err == nil {
		tracking.SetFinalizer("CachedTexture", texture)
	}
	if settings.Performance().ShouldCacheImages() {
		return i.cache.Set(key, texture, ttlcache.DefaultTTL)
	}
	return ttlcache.NewItemWithOpts(key, texture, ttlcache.DefaultTTL)
}

func (i *ImgUtil) Fetch(url string) (*gdk.Texture, error) {
	item := i.cache.Get(url)
	if item == nil {
		return nil, nil
	}

	return item.Value(), nil
}

func (i *ImgUtil) FetchCropped(url string) (*gdk.Texture, error) {
	item := i.cache.Get(url)
	if item == nil {
		return nil, nil
	}

	texture := item.Value()
	cropped := Crop(texture)

	tracking.SetFinalizer("Texture", cropped)

	return cropped, nil
}
