package resources

import (
	"codeberg.org/dergs/tonearm/internal/g"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/schwifty/tracking"
	"codeberg.org/puregotk/puregotk/v4/gdk"
)

var MissingAlbum = g.Lazy(func() schwifty.Paintable {
	image := gdk.NewTextureFromResource("/dev/dergs/Tonearm/icons/hicolor/512x512/state/missing-album.png")
	image.Ref()
	tracking.SetFinalizer("Texture", image)
	return image
})
