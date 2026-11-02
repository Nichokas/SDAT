package imgutil

import (
	"math"

	"codeberg.org/puregotk/puregotk/v4/gdk"
)

func Crop(texture *gdk.Texture) *gdk.Texture {
	texture.Ref()
	defer texture.Unref()

	size := int32(math.Min(float64(texture.GetIntrinsicWidth()), float64(texture.GetIntrinsicHeight())))
	src_x := (texture.GetIntrinsicWidth() - size) / 2
	src_y := (texture.GetIntrinsicHeight() - size) / 2

	pixbuf := gdk.PixbufGetFromTexture(texture)
	defer pixbuf.Unref()

	cropped := pixbuf.NewSubpixbuf(src_x, src_y, size, size)
	defer cropped.Unref()

	return gdk.NewTextureForPixbuf(cropped)
}
